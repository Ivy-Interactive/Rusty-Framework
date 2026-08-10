//! `rusty/examples/counter.rs`, with the UI in XAML instead of builder calls.
//!
//! Run it with `cargo run -p rusty-xaml --example xaml_counter`, then open
//! <http://127.0.0.1:3000>.
//!
//! The point of interest is where the parse happens: *inside* `build`, against a
//! context rebuilt from the current state. A binding is resolved once, when the
//! document is parsed, so parsing per build is what makes `{Binding Count}` track
//! the counter — see the crate docs. The markup itself could equally come from a
//! file with `parse_file_with`, which is the case this crate exists for.

use rusty::prelude::*;
use rusty_xaml::XamlContext;

/// Hand-written here to keep the example self-contained; a real app would load
/// this from disk, from a database, or from an editor.
const MARKUP: &str = r#"
<StackPanel Spacing="16" Padding="24">
    <TextBlock Text="XAML Counter" Variant="Heading1" />
    <TextBlock Text="{Binding CountLabel}" />

    <StackPanel Orientation="Horizontal" Spacing="8">
        <Button Content="Increment" Click="OnIncrement" />
        <Button Content="Decrement" Variant="Secondary" Click="OnDecrement" />
        <Button Content="Reset" Variant="Ghost" Click="OnReset" />
    </StackPanel>

    <Separator />

    <ProgressBar Value="{Binding Progress}" Label="Progress to 10" />
</StackPanel>
"#;

struct XamlCounterApp;

#[rusty::view]
impl View for XamlCounterApp {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let count = use_state(ctx, 0i32);

        let count_inc = count.clone();
        let count_dec = count.clone();
        let count_reset = count.clone();

        let xaml = XamlContext::new()
            .value("CountLabel", format!("Count: {}", count.get()))
            .value("Progress", f64::from(count.get().clamp(0, 10)) / 10.0)
            .handler("OnIncrement", move || {
                count_inc.update(|v| v + 1);
            })
            .handler("OnDecrement", move || {
                count_dec.update(|v| v - 1);
            })
            .handler("OnReset", move || {
                count_reset.set(0);
            });

        // A runtime parser can fail at runtime, so the app has to say something
        // rather than panic: the error already names the element, the attribute
        // and the line it came from.
        match rusty_xaml::parse_with(MARKUP, &xaml) {
            Ok(element) => element,
            Err(err) => Layout::vertical()
                .gap(8.0)
                .padding(24.0)
                .child(TextBlock::h1("The markup did not parse"))
                .child(TextBlock::code(&err.to_string()))
                .into(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    // `RustyServer` binds `DEFAULT_BIND_ADDRESS`, which is loopback.
    RustyServer::new(port, || XamlCounterApp).serve().await
}
