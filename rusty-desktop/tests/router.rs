//! `desktop_router` serves the embedded client without a static directory, and does not
//! displace `rusty`'s own routes.
//!
//! These tests must pass with `--no-default-features` — that is what CI runs — so nothing
//! here touches the `shell` feature.

use std::net::SocketAddr;

use axum::Router;
use rusty::prelude::{BuildContext, Element, Layout, RustyServer, TextBlock, View};
use rusty_desktop::desktop_router;

struct Probe;

impl View for Probe {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        Layout::vertical().child(TextBlock::new("probe")).into()
    }
}

/// `RustyServer::serve_background` builds its own listener from the configured address.
/// Binding here keeps the test on an ephemeral loopback port and lets the router under
/// test be the one `desktop_router` produced.
async fn serve_on_loopback(router: Router) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback");
    let addr = listener.local_addr().expect("local_addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    addr
}

async fn get(addr: SocketAddr, path: &str) -> (u16, String) {
    // A raw HTTP/1.0 request keeps this test free of an HTTP-client dev-dependency;
    // HTTP/1.0 has no keep-alive, so the server closes and `read_to_end` terminates.
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut stream = tokio::net::TcpStream::connect(addr).await.expect("connect");
    let request = format!("GET {path} HTTP/1.0\r\nHost: {addr}\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .await
        .expect("write request");
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .expect("read response");
    let response = String::from_utf8_lossy(&response).into_owned();

    let status = response
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status line in {response:?}"));
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_string())
        .unwrap_or_default();

    (status, body)
}

#[tokio::test]
async fn root_serves_the_embedded_client() {
    let addr = serve_on_loopback(desktop_router(RustyServer::new(0, || Probe))).await;

    let (status, body) = get(addr, "/").await;
    assert_eq!(status, 200);
    assert!(
        body.contains("new WebSocket"),
        "root did not serve the renderer: {body:.200?}"
    );
    // No static dir was configured, so this proves the bytes came from the binary.
    assert!(body.contains("<!DOCTYPE html>"));
}

#[tokio::test]
async fn layered_root_does_not_displace_the_framework_routes() {
    let addr = serve_on_loopback(desktop_router(RustyServer::new(0, || Probe))).await;

    let (status, body) = get(addr, "/health").await;
    assert_eq!(status, 200);
    assert_eq!(body, "ok");
}

#[tokio::test]
async fn unknown_paths_are_not_captured_by_the_root_route() {
    // `route("/")` is an exact match, not a fallback: an unknown path must still 404 rather
    // than quietly serving the client, which would mask a broken asset URL.
    let addr = serve_on_loopback(desktop_router(RustyServer::new(0, || Probe))).await;

    let (status, _) = get(addr, "/does-not-exist").await;
    assert_eq!(status, 404);
}
