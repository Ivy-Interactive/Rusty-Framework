use clap::{Parser, ValueEnum};
use rusty::prelude::*;
use rusty::widgets::badge::BadgeVariant;
use rusty::widgets::button::ButtonVariant;
use rusty::widgets::input::SelectOption;
use rusty::widgets::table::Column;
use serde_json::json;
use std::path::PathBuf;

/// One variant per widget the harness can serve. The snake_case names are the
/// contract with `e2e/tests/harness.ts`, which passes them straight through as
/// the first argv.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "snake_case")]
enum WidgetKind {
    Button,
    Text,
    TextInput,
    NumberInput,
    Select,
    Checkbox,
    Layout,
    Card,
    Query,
    Badge,
    Progress,
    Table,
    Dialog,
    Tooltip,
}

#[derive(Parser)]
#[command(name = "widget_harness")]
#[command(about = "Launch a minimal Rusty app exercising a single widget for E2E testing")]
struct Cli {
    /// Widget to test
    widget: WidgetKind,

    /// Port to listen on (0 for auto-assign)
    #[arg(short, long, default_value = "0", env = "PORT")]
    port: u16,

    /// Directory to serve static files from
    #[arg(short, long)]
    static_dir: Option<PathBuf>,
}

impl WidgetKind {
    /// The single dispatch point from a kind to its sample app. Because the match
    /// is exhaustive, a variant added without an arm fails to compile.
    fn build_app(self, ctx: &mut BuildContext) -> Element {
        match self {
            WidgetKind::Button => ButtonApp.build(ctx),
            WidgetKind::Text => TextApp.build(ctx),
            WidgetKind::TextInput => TextInputApp.build(ctx),
            WidgetKind::NumberInput => NumberInputApp.build(ctx),
            WidgetKind::Select => SelectApp.build(ctx),
            WidgetKind::Checkbox => CheckboxApp.build(ctx),
            WidgetKind::Layout => LayoutApp.build(ctx),
            WidgetKind::Card => CardApp.build(ctx),
            WidgetKind::Query => QueryApp.build(ctx),
            WidgetKind::Badge => BadgeApp.build(ctx),
            WidgetKind::Progress => ProgressApp.build(ctx),
            WidgetKind::Table => TableApp.build(ctx),
            WidgetKind::Dialog => DialogApp.build(ctx),
            WidgetKind::Tooltip => TooltipApp.build(ctx),
        }
    }
}

/// The root view the server serves: whichever sample app the kind selects.
struct HarnessApp(WidgetKind);

impl View for HarnessApp {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        self.0.build_app(ctx)
    }
}
struct ButtonApp;

impl View for ButtonApp {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let count = use_state(ctx, 0i32);
        let count_val = count.get();
        let count_clone = count.clone();

        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("Button Test"))
            .child(
                Button::new("Click me")
                    .variant(ButtonVariant::Primary)
                    .on_click(move || {
                        count_clone.update(|n| n + 1);
                    }),
            )
            .child(Button::new("Secondary").variant(ButtonVariant::Secondary))
            .child(Button::new("Disabled").disabled(true))
            .child(TextBlock::paragraph(&format!("Count: {}", count_val)))
            .into()
    }
}

struct TextApp;

impl View for TextApp {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        Layout::vertical()
            .gap(8.0)
            .child(TextBlock::h1("Heading 1"))
            .child(TextBlock::h2("Heading 2"))
            .child(TextBlock::h3("Heading 3"))
            .child(TextBlock::paragraph("This is a paragraph."))
            .child(TextBlock::code("let x = 42;"))
            .child(TextBlock::label("A label"))
            .into()
    }
}

struct TextInputApp;

impl View for TextInputApp {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let text_val = use_state(ctx, "hello".to_string());
        let text_display = text_val.get();
        let text_clone = text_val.clone();

        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("TextInput Test"))
            .child(
                TextInput::new()
                    .label("Name")
                    .placeholder("Enter text")
                    .value(&text_display)
                    .on_change(move |v: String| {
                        text_clone.set(v);
                    }),
            )
            .child(TextBlock::paragraph(&format!("Value: {}", text_display)))
            .into()
    }
}

struct NumberInputApp;

impl View for NumberInputApp {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let num_val = use_state(ctx, 42.0f64);
        let num_display = num_val.get();
        let num_clone = num_val.clone();

        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("NumberInput Test"))
            .child(
                NumberInput::new()
                    .label("Amount")
                    .value(num_display)
                    .min(0.0)
                    .max(100.0)
                    .step(1.0)
                    .on_change(move |v: f64| {
                        num_clone.set(v);
                    }),
            )
            .child(TextBlock::paragraph(&format!("Value: {}", num_display)))
            .into()
    }
}

struct SelectApp;

impl View for SelectApp {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let selected = use_state(ctx, "apple".to_string());
        let selected_display = selected.get();
        let selected_clone = selected.clone();

        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("Select Test"))
            .child(
                Select::new(vec![
                    SelectOption {
                        value: "apple".into(),
                        label: "Apple".into(),
                    },
                    SelectOption {
                        value: "banana".into(),
                        label: "Banana".into(),
                    },
                    SelectOption {
                        value: "cherry".into(),
                        label: "Cherry".into(),
                    },
                ])
                .label("Fruit")
                .value(&selected_display)
                .on_change(move |v: String| {
                    selected_clone.set(v);
                }),
            )
            .child(TextBlock::paragraph(&format!(
                "Selected: {}",
                selected_display
            )))
            .into()
    }
}

struct CheckboxApp;

impl View for CheckboxApp {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let checked = use_state(ctx, false);
        let checked_val = checked.get();
        let checked_clone = checked.clone();

        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("Checkbox Test"))
            .child(
                Checkbox::new(checked_val)
                    .label("Accept terms")
                    .on_change(move |v: bool| {
                        checked_clone.set(v);
                    }),
            )
            .child(TextBlock::paragraph(&format!("Checked: {}", checked_val)))
            .into()
    }
}

struct LayoutApp;

impl View for LayoutApp {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("Layout Test"))
            .child(
                Layout::horizontal()
                    .gap(8.0)
                    .child(Button::new("Left"))
                    .child(Button::new("Center"))
                    .child(Button::new("Right")),
            )
            .child(
                Layout::grid(3)
                    .gap(8.0)
                    .child(TextBlock::paragraph("Cell 1"))
                    .child(TextBlock::paragraph("Cell 2"))
                    .child(TextBlock::paragraph("Cell 3"))
                    .child(TextBlock::paragraph("Cell 4"))
                    .child(TextBlock::paragraph("Cell 5"))
                    .child(TextBlock::paragraph("Cell 6")),
            )
            .into()
    }
}

struct CardApp;

impl View for CardApp {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("Card Test"))
            .child(
                Card::new()
                    .title("My Card")
                    .subtitle("A subtitle")
                    .child(TextBlock::paragraph("Card body content")),
            )
            .child(
                Card::new()
                    .title("Another Card")
                    .child(Button::new("Card Action")),
            )
            .into()
    }
}

struct BadgeApp;

impl View for BadgeApp {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("Badge Test"))
            .child(Badge::new("Default").variant(BadgeVariant::Default))
            .child(Badge::new("Outline").variant(BadgeVariant::Outline))
            .child(Badge::new("Dot").variant(BadgeVariant::Dot))
            .child(Badge::new("Success").color(Color::Named(NamedColor::Success)))
            .child(Badge::new("Warning").color(Color::Named(NamedColor::Warning)))
            .child(Badge::new("Danger").color(Color::Named(NamedColor::Danger)))
            .into()
    }
}

struct ProgressApp;

impl View for ProgressApp {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        // 0.25 steps are exactly representable in binary floating point, so the
        // readout stays free of 0.30000000000000004-style noise.
        let value = use_state(ctx, 0.0f64);
        let value_display = value.get();
        let value_clone = value.clone();

        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("Progress Test"))
            .child(Progress::new(0.25))
            .child(Progress::new(0.75).label("Upload progress"))
            .child(Progress::indeterminate())
            .child(Progress::new(50.0).max(200.0))
            .child(Progress::new(value_display).label("Advancing"))
            .child(Button::new("Advance").on_click(move || {
                value_clone.update(|v| (v + 0.25).min(1.0));
            }))
            .child(TextBlock::paragraph(&format!("Value: {}", value_display)))
            .into()
    }
}

struct TableApp;

impl View for TableApp {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        let columns = vec![
            Column {
                key: "name".into(),
                label: "Name".into(),
                sortable: true,
            },
            Column {
                key: "role".into(),
                label: "Role".into(),
                sortable: false,
            },
            Column {
                key: "age".into(),
                label: "Age".into(),
                sortable: true,
            },
        ];

        let rows = vec![
            json!({ "name": "Ada", "role": "Engineer", "age": 36 }),
            json!({ "name": "Grace", "role": "Admiral", "age": 45 }),
            json!({ "name": "Alan", "role": "Researcher", "age": 41 }),
        ];

        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("Table Test"))
            .child(Table::new(columns.clone()).rows(rows.clone()))
            .child(Table::new(columns).rows(rows).sort_by("name", true))
            .into()
    }
}

struct DialogApp;

impl View for DialogApp {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let open = use_state(ctx, false);
        let open_val = open.get();
        let open_clone = open.clone();
        let close_clone = open.clone();

        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("Dialog Test"))
            .child(Button::new("Open dialog").on_click(move || {
                open_clone.set(true);
            }))
            .child(
                Dialog::new(open_val)
                    .title("Confirm action")
                    .child(TextBlock::paragraph("Are you sure about this?"))
                    .footer(vec![Button::new("Close")
                        .variant(ButtonVariant::Secondary)
                        .on_click(move || {
                            close_clone.set(false);
                        })
                        .into()]),
            )
            .child(TextBlock::paragraph(&format!("Open: {}", open_val)))
            .into()
    }
}

struct TooltipApp;

impl View for TooltipApp {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("Tooltip Test"))
            .child(Tooltip::new("Buttons do things", Button::new("Hover me")))
            .child(Tooltip::new(
                "Text can be explained too",
                TextBlock::paragraph("Hover this text"),
            ))
            .into()
    }
}

/// Exercises `use_query`. The fetcher sleeps, so the loading-to-loaded
/// transition arrives over the WebSocket push path rather than in the first
/// render.
struct QueryApp;

impl View for QueryApp {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let result = use_query(
            ctx,
            Some("harness-greeting"),
            || async {
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                Ok("Hello from the query cache".to_string())
            },
            QueryOptions::default(),
        );

        let status = if result.loading {
            "Loading...".to_string()
        } else if let Some(error) = &result.error {
            format!("Error: {}", error)
        } else {
            result.value.clone().unwrap_or_default()
        };
        let validating = if result.validating {
            "validating"
        } else {
            "idle"
        };
        let mutator = result.mutator.clone();

        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("Query Test"))
            .child(TextBlock::paragraph(&status))
            .child(TextBlock::label(validating))
            .child(
                Button::new("Revalidate")
                    .variant(ButtonVariant::Primary)
                    .on_click(move || mutator.revalidate()),
            )
            .into()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    // clap rejects unknown widget names before we get here, so there is no
    // unknown-widget arm to write.
    let widget = cli.widget;
    let server = RustyServer::new(cli.port, move || HarnessApp(widget));

    let server = if let Some(dir) = cli.static_dir {
        server.with_static_dir(dir)
    } else {
        server
    };

    server.serve().await
}
