//! The native window, WebView and menu bar.
//!
//! Everything in this module is an uncovered shim by construction, and that is why the
//! interesting logic lives in [`crate::menu`] instead:
//!
//! - `tao` panics when an event loop is built off the main thread ("Initializing the event
//!   loop outside of the main thread is a significant cross-platform compatibility
//!   hazard"), and `cargo test` runs every test on a spawned thread. No `#[test]` can
//!   construct the loop.
//! - `muda::MenuEvent::send` is `pub(crate)`, so no test can synthesize a menu click.
//!
//! This module is behind the default-on `shell` feature: on Linux, `wry` pulls
//! `webkit2gtk-sys` and friends, which need `libwebkit2gtk-4.1-dev` — not present on
//! `ubuntu-latest`. CI builds the workspace `--no-default-features` and covers this module
//! in a separate `windows-latest` job.

use std::net::SocketAddr;

use axum::Router;
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use wry::WebViewBuilder;

use crate::menu::{action_for_id, MenuAction, MenuEntry, MenuSpec};

/// Window title used when the caller passes none.
pub const DEFAULT_TITLE: &str = "Rusty";

/// Configuration for [`run`].
pub struct ShellOptions {
    /// Native window title.
    pub title: String,
    /// Whether the WebView should permit the inspector.
    ///
    /// `wry` only compiles `open_devtools` under `debug_assertions` or its own `devtools`
    /// feature, so in a release build without `--features devtools` this is inert.
    pub devtools: bool,
}

impl Default for ShellOptions {
    fn default() -> Self {
        ShellOptions {
            title: DEFAULT_TITLE.to_string(),
            devtools: cfg!(debug_assertions),
        }
    }
}

/// Start the embedded server, open the window, and run the event loop until the user quits.
///
/// Must be called on the main thread — `tao` panics otherwise. Never returns normally: the
/// process exits from inside the event loop.
pub fn run(
    spec: MenuSpec,
    router: Router,
    options: ShellOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    // The runtime must outlive the event loop: dropping it aborts the spawned server task
    // and every subsequent request fails with a connection refused.
    let runtime = tokio::runtime::Runtime::new()?;
    let addr = runtime.block_on(async move { spawn_server(router).await })?;
    tracing::info!("Embedded Rusty server listening on {}", addr);

    // A desktop app must not be reachable from the LAN, so loopback + port 0, matching
    // `rusty::prelude::DEFAULT_BIND_ADDRESS`.
    let event_loop = EventLoopBuilder::<MenuAction>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let menu = build_menu(&spec)?;

    // The handler fires on the platform's menu thread, so it only translates the id and
    // forwards; the action itself runs on the loop thread below.
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        match action_for_id(&spec, &event.id.0) {
            Some(action) => {
                let _ = proxy.send_event(action);
            }
            // The platform can report items we never registered; a menu click must not
            // take the process down.
            None => tracing::debug!("unbound menu id {:?}", event.id.0),
        }
    }));

    let window = WindowBuilder::new()
        .with_title(options.title)
        .build(&event_loop)?;

    attach_menu(&menu, &window)?;

    let url = format!("http://{addr}/");
    // `webview` is moved into the `run` closure: dropping it closes the WebView.
    let webview = WebViewBuilder::new()
        .with_url(&url)
        .with_devtools(options.devtools)
        .build(&window)?;

    // `event_loop.run` returns `!`, so nothing after this line ever executes.
    event_loop.run(move |event, _target, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(action) => match action {
                MenuAction::Reload => {
                    if let Err(error) = webview.evaluate_script("location.reload()") {
                        tracing::warn!("reload failed: {error}");
                    }
                }
                MenuAction::ToggleDevTools => toggle_devtools(&webview),
                MenuAction::NavigateTo(app_id) => {
                    // The renderer reads `?appId=` and passes it on the WebSocket URL.
                    let target = format!("http://{addr}/?appId={}", encode_query(&app_id));
                    if let Err(error) = webview.load_url(&target) {
                        tracing::warn!("navigation to {app_id} failed: {error}");
                    }
                }
                MenuAction::Quit => *control_flow = ControlFlow::Exit,
            },
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    })
}

/// Bind an ephemeral loopback port and serve `router` on a background task.
async fn spawn_server(router: Router) -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, router).await {
            tracing::error!("embedded server stopped: {error}");
        }
    });
    Ok(addr)
}

/// Walk a [`MenuSpec`] into a `muda::Menu`.
fn build_menu(spec: &MenuSpec) -> Result<Menu, Box<dyn std::error::Error>> {
    let menu = Menu::new();

    for (label, entries) in &spec.submenus {
        let submenu = Submenu::new(label, true);
        for entry in entries {
            match entry {
                MenuEntry::Item { id, label, .. } => {
                    submenu.append(&MenuItem::with_id(id.as_str(), label, true, None))?;
                }
                MenuEntry::Separator => submenu.append(&PredefinedMenuItem::separator())?,
                MenuEntry::Quit => submenu.append(&PredefinedMenuItem::quit(Some("Quit")))?,
            }
        }
        menu.append(&submenu)?;
    }

    Ok(menu)
}

/// Attach the menu bar to the window. There is no portable call for this.
#[cfg(target_os = "windows")]
fn attach_menu(
    menu: &Menu,
    window: &tao::window::Window,
) -> Result<(), Box<dyn std::error::Error>> {
    use tao::platform::windows::WindowExtWindows;

    // SAFETY: `hwnd()` returns this window's live HWND, and the window outlives the menu
    // because both are dropped when `run`'s closure is dropped at process exit.
    unsafe { menu.init_for_hwnd(window.hwnd())? };
    Ok(())
}

#[cfg(target_os = "macos")]
fn attach_menu(
    menu: &Menu,
    _window: &tao::window::Window,
) -> Result<(), Box<dyn std::error::Error>> {
    // macOS owns one application-wide menu bar, not a per-window one.
    menu.init_for_nsapp();
    Ok(())
}

#[cfg(target_os = "linux")]
fn attach_menu(
    menu: &Menu,
    window: &tao::window::Window,
) -> Result<(), Box<dyn std::error::Error>> {
    use tao::platform::unix::WindowExtUnix;

    menu.init_for_gtk_window(window.gtk_window(), window.default_vbox())?;
    Ok(())
}

/// `wry` compiles the inspector API only under `debug_assertions` or its own `devtools`
/// feature, so a release build without `--features devtools` has nothing to toggle.
#[cfg(any(debug_assertions, feature = "devtools"))]
fn toggle_devtools(webview: &wry::WebView) {
    // `is_devtools_open` and `close_devtools` are documented as unsupported on Windows,
    // where the inspector is a WebView2-owned window the user closes themselves. Opening
    // is the portable half.
    if webview.is_devtools_open() {
        webview.close_devtools();
    } else {
        webview.open_devtools();
    }
}

#[cfg(not(any(debug_assertions, feature = "devtools")))]
fn toggle_devtools(_webview: &wry::WebView) {
    tracing::info!("built without devtools support; rebuild with --features devtools");
}

/// Percent-encode the characters that would break out of a query value.
///
/// App ids come from the registry, not from user input, but an id containing `&`, `#` or a
/// space would silently truncate the URL.
fn encode_query(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'$' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // These are the only assertions this module can carry: everything else here needs a
    // main-thread event loop. See the module docs.

    #[test]
    fn encode_query_passes_ordinary_ids_through() {
        // `$default` is a reserved id in `rusty::prelude::AppIds`, so `$` must survive.
        assert_eq!(encode_query("$default"), "$default");
        assert_eq!(encode_query("reports-v2_1.0~x"), "reports-v2_1.0~x");
    }

    #[test]
    fn encode_query_escapes_url_delimiters() {
        assert_eq!(encode_query("a&b"), "a%26b");
        assert_eq!(encode_query("a b"), "a%20b");
        assert_eq!(encode_query("a#b"), "a%23b");
    }

    #[test]
    fn default_options_enable_devtools_only_in_debug_builds() {
        let options = ShellOptions::default();
        assert_eq!(options.title, DEFAULT_TITLE);
        assert_eq!(options.devtools, cfg!(debug_assertions));
    }
}
