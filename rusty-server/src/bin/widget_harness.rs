use clap::Parser;
use rusty::prelude::*;
use rusty::widgets::button::ButtonVariant;
use rusty::widgets::input::SelectOption;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "widget_harness")]
#[command(about = "Launch a minimal Rusty app exercising a single widget for E2E testing")]
struct Cli {
    /// Widget to test (button, text, text_input, number_input, select, checkbox, layout, card,
    /// data_table, form, diff_view, qr_code, activity_heatmap, terminal, rich_text_input)
    widget: String,

    /// Port to listen on (0 for auto-assign)
    #[arg(short, long, default_value = "0")]
    port: u16,

    /// Directory to serve static files from
    #[arg(short, long)]
    static_dir: Option<PathBuf>,
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

struct DataTableApp;

impl View for DataTableApp {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let last_cell = use_state(ctx, "none".to_string());
        let last_cell_display = last_cell.get();
        let last_cell_clone = last_cell.clone();

        let columns = vec![
            DataTableColumn::new("name", "Name", ColType::Text),
            DataTableColumn::new("age", "Age", ColType::Number).align(Align::End),
            DataTableColumn::new("active", "Active", ColType::Boolean),
        ];

        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("DataTable Test"))
            .child(
                DataTable::new(columns)
                    .rows(vec![
                        serde_json::json!({"name": "Alice", "age": 30, "active": true}),
                        serde_json::json!({"name": "Bob", "age": 25, "active": false}),
                    ])
                    .config(DataTableConfig::new().show_search(true))
                    .on_cell_click(move |args| {
                        last_cell_clone.set(args.column_name);
                    }),
            )
            .child(TextBlock::paragraph(&format!(
                "Last cell: {}",
                last_cell_display
            )))
            .into()
    }
}

#[derive(Clone, Default)]
struct Signup {
    name: String,
    email: String,
}

struct FormApp;

impl View for FormApp {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let builder = FormBuilder::<Signup>::new()
            .field(
                "name",
                "Name",
                std::sync::Arc::new(|model: &Signup, set: ModelSetter<Signup>| {
                    let current = model.clone();
                    TextInput::new()
                        .placeholder("Your name")
                        .value(&model.name)
                        .on_change(move |v: String| {
                            let mut next = current.clone();
                            next.name = v;
                            set(next);
                        })
                        .into()
                }),
            )
            .field(
                "email",
                "Email",
                std::sync::Arc::new(|model: &Signup, set: ModelSetter<Signup>| {
                    let current = model.clone();
                    TextInput::new()
                        .placeholder("you@example.com")
                        .value(&model.email)
                        .on_change(move |v: String| {
                            let mut next = current.clone();
                            next.email = v;
                            set(next);
                        })
                        .into()
                }),
            )
            .required("name")
            .description("email", "We never share it")
            .validate(
                "name",
                std::sync::Arc::new(|m: &Signup| rusty::views::validators::not_empty(&m.name)),
            )
            .validate(
                "email",
                std::sync::Arc::new(|m: &Signup| rusty::views::validators::email(&m.email)),
            )
            .submit_title("Sign up");

        let (model, _errors, form) = use_form(ctx, Signup::default(), builder);
        let current = model.get();

        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("Form Test"))
            .child(form)
            .child(TextBlock::paragraph(&format!(
                "Name: {} / Email: {}",
                current.name, current.email
            )))
            .into()
    }
}

struct DiffViewApp;

impl View for DiffViewApp {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        let diff =
            "@@ -1,3 +1,3 @@\n fn main() {\n-    println!(\"old\");\n+    println!(\"new\");\n }\n";

        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("DiffView Test"))
            .child(
                DiffView::new()
                    .diff(diff)
                    .language("rust")
                    .old_revision("HEAD~1")
                    .new_revision("HEAD")
                    .collapsible(true)
                    .on_line_click(|_line| {}),
            )
            .into()
    }
}

struct QrCodeApp;

impl View for QrCodeApp {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("QrCode Test"))
            .child(
                QrCode::new("https://example.com")
                    .pixel_size(6)
                    .error_correction_level(QrErrorCorrectionLevel::Medium),
            )
            .into()
    }
}

struct ActivityHeatmapApp;

impl View for ActivityHeatmapApp {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("ActivityHeatmap Test"))
            .child(
                ActivityHeatmap::new()
                    .data(vec![
                        Activity::new("2026-01-01", 3),
                        Activity::new("2026-01-02", 7),
                        Activity::new("2026-01-03", 1),
                    ])
                    .value_label("commits")
                    .start_date("2026-01-01")
                    .end_date("2026-01-31")
                    .on_day_click(|_activity| {}),
            )
            .into()
    }
}

struct TerminalApp;

impl View for TerminalApp {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("Terminal Test"))
            .child(
                Terminal::new()
                    .cols(80)
                    .rows(24)
                    .cursor_style(CursorStyle::Bar)
                    .initial_content("$ echo hello\nhello\n")
                    .on_input(|_data| {})
                    .on_resize(|_size| {})
                    .on_link_click(|_url| {}),
            )
            .into()
    }
}

struct RichTextInputApp;

impl View for RichTextInputApp {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let html = use_state(ctx, "<p>Hello</p>".to_string());
        let html_display = html.get();
        let html_clone = html.clone();

        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("RichTextInput Test"))
            .child(
                RichTextInput::new()
                    .value(&html_display)
                    .placeholder("Write something…")
                    .on_change(move |v: String| {
                        html_clone.set(v);
                    })
                    .on_focus(|| {})
                    .on_blur(|| {}),
            )
            .child(TextBlock::paragraph(&format!("Value: {}", html_display)))
            .into()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    let widget = cli.widget.as_str();
    let port = cli.port;
    let static_dir = cli.static_dir;

    let server = match widget {
        "button" => RustyServer::new(port, || ButtonApp),
        "text" => RustyServer::new(port, || TextApp),
        "text_input" => RustyServer::new(port, || TextInputApp),
        "number_input" => RustyServer::new(port, || NumberInputApp),
        "select" => RustyServer::new(port, || SelectApp),
        "checkbox" => RustyServer::new(port, || CheckboxApp),
        "layout" => RustyServer::new(port, || LayoutApp),
        "card" => RustyServer::new(port, || CardApp),
        "data_table" => RustyServer::new(port, || DataTableApp),
        "form" => RustyServer::new(port, || FormApp),
        "diff_view" => RustyServer::new(port, || DiffViewApp),
        "qr_code" => RustyServer::new(port, || QrCodeApp),
        "activity_heatmap" => RustyServer::new(port, || ActivityHeatmapApp),
        "terminal" => RustyServer::new(port, || TerminalApp),
        "rich_text_input" => RustyServer::new(port, || RichTextInputApp),
        other => {
            eprintln!("Unknown widget: {}", other);
            std::process::exit(1);
        }
    };

    let server = if let Some(dir) = static_dir {
        server.with_static_dir(dir)
    } else {
        server
    };

    server.serve().await
}
