use clap::Parser;
use rusty::prelude::*;
use rusty::widgets::button::ButtonVariant;
use rusty::widgets::input::SelectOption;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "widget_harness")]
#[command(about = "Launch a minimal Rusty app exercising a single widget for E2E testing")]
struct Cli {
    /// Widget to test. Run with an unknown name to list the accepted values.
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

struct SpacerApp;

impl View for SpacerApp {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        Layout::horizontal()
            .gap(0.0)
            .child(TextBlock::paragraph("Left"))
            .child(Spacer::new())
            .child(TextBlock::paragraph("Right"))
            .into()
    }
}

struct SeparatorApp;

impl View for SeparatorApp {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        Layout::vertical()
            .gap(8.0)
            .child(TextBlock::h1("Separator Test"))
            .child(Separator::horizontal())
            .child(Separator::horizontal().text("OR"))
            .child(
                Layout::horizontal()
                    .gap(8.0)
                    .child(TextBlock::paragraph("Before"))
                    .child(Separator::vertical())
                    .child(TextBlock::paragraph("After")),
            )
            .into()
    }
}

struct ContainerApp;

impl View for ContainerApp {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("Container Test"))
            .child(
                Container::new()
                    .padding(16.0)
                    .border(true)
                    .rounded(true)
                    .child(TextBlock::paragraph("Bordered and rounded")),
            )
            .child(
                Container::new()
                    .width(Size::Px(200.0))
                    .height(Size::Px(80.0))
                    .background(Color::hex("#eef4ff"))
                    .child(TextBlock::paragraph("Fixed size")),
            )
            .into()
    }
}

struct IconApp;

impl View for IconApp {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        Layout::horizontal()
            .gap(8.0)
            .child(IconWidget::new("check"))
            .child(IconWidget::new("alert").size(32.0))
            .child(IconWidget::new("info").color(Color::hex("#0066cc")))
            .into()
    }
}

struct ImageApp;

impl View for ImageApp {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        // A data URI keeps the harness from reaching out over the network.
        let pixel =
            "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";

        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("Image Test"))
            .child(Image::new(pixel).alt("A transparent pixel"))
            .child(
                Image::new(pixel)
                    .alt("Sized pixel")
                    .width(Size::Px(64.0))
                    .height(Size::Px(64.0)),
            )
            .into()
    }
}

struct AvatarApp;

impl View for AvatarApp {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        Layout::horizontal()
            .gap(8.0)
            .child(Avatar::new("AB"))
            .child(Avatar::new("CD").size(Density::Compact))
            .child(Avatar::new("EF").size(Density::Comfortable))
            .into()
    }
}

struct CalloutApp;

impl View for CalloutApp {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("Callout Test"))
            .child(
                Callout::info()
                    .title("Heads up")
                    .child(TextBlock::paragraph("An informational note.")),
            )
            .child(Callout::success().child(TextBlock::paragraph("It worked.")))
            .child(Callout::warning().child(TextBlock::paragraph("Careful.")))
            .child(Callout::error().child(TextBlock::paragraph("It broke.")))
            .into()
    }
}

struct SkeletonApp;

impl View for SkeletonApp {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        Layout::vertical()
            .gap(8.0)
            .child(
                Skeleton::new()
                    .width(Size::Px(240.0))
                    .height(Size::Px(16.0)),
            )
            .child(
                Skeleton::new()
                    .width(Size::Percent(60.0))
                    .height(Size::Px(16.0)),
            )
            .into()
    }
}

struct ExpandableApp;

impl View for ExpandableApp {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let expanded = use_state(ctx, false);
        let expanded_val = expanded.get();
        let expanded_clone = expanded.clone();

        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("Expandable Test"))
            .child(
                Expandable::new("Details")
                    .expanded(expanded_val)
                    .child(TextBlock::paragraph("Hidden body content"))
                    .on_toggle(move |value: bool| {
                        expanded_clone.set(value);
                    }),
            )
            .child(TextBlock::paragraph(&format!("Expanded: {}", expanded_val)))
            .into()
    }
}

struct ListApp;

impl View for ListApp {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let selected = use_state(ctx, String::new());
        let selected_val = selected.get();
        let inbox = selected.clone();
        let drafts = selected.clone();

        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("List Test"))
            .child(
                List::new()
                    .item(
                        ListItem::new("Inbox")
                            .subtitle("3 unread")
                            .icon("mail")
                            .on_click(move || inbox.set("Inbox".to_string())),
                    )
                    .item(
                        ListItem::new("Drafts").on_click(move || drafts.set("Drafts".to_string())),
                    )
                    .item(ListItem::new("Archive")),
            )
            .child(TextBlock::paragraph(&format!("Selected: {}", selected_val)))
            .into()
    }
}

struct TextAreaApp;

impl View for TextAreaApp {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let value = use_state(ctx, String::new());
        let value_val = value.get();
        let value_clone = value.clone();

        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("TextArea Test"))
            .child(
                TextArea::new()
                    .label("Message")
                    .placeholder("Say something")
                    .rows(4)
                    .value(&value_val)
                    .on_change(move |v: String| {
                        value_clone.set(v);
                    }),
            )
            .child(TextBlock::paragraph(&format!("Value: {}", value_val)))
            .into()
    }
}

struct SliderApp;

impl View for SliderApp {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let value = use_state(ctx, 25.0f64);
        let value_val = value.get();
        let value_clone = value.clone();

        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("Slider Test"))
            .child(
                Slider::new(value_val)
                    .label("Volume")
                    .min(0.0)
                    .max(100.0)
                    .step(5.0)
                    .on_change(move |v: f64| {
                        value_clone.set(v);
                    }),
            )
            .child(TextBlock::paragraph(&format!("Value: {}", value_val)))
            .into()
    }
}

struct DateInputApp;

impl View for DateInputApp {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let value = use_state(ctx, String::new());
        let value_val = value.get();
        let value_clone = value.clone();

        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("DateInput Test"))
            .child(
                DateInput::new()
                    .label("Due date")
                    .min("2026-01-01")
                    .max("2026-12-31")
                    .value(&value_val)
                    .on_change(move |v: String| {
                        value_clone.set(v);
                    }),
            )
            .child(TextBlock::paragraph(&format!("Value: {}", value_val)))
            .into()
    }
}

struct ColorInputApp;

impl View for ColorInputApp {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let value = use_state(ctx, "#000000".to_string());
        let value_val = value.get();
        let value_clone = value.clone();

        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("ColorInput Test"))
            .child(
                ColorInput::new()
                    .label("Accent")
                    .value(&value_val)
                    .on_change(move |v: String| {
                        value_clone.set(v);
                    }),
            )
            .child(TextBlock::paragraph(&format!("Value: {}", value_val)))
            .into()
    }
}

fn size_options() -> Vec<SelectOption> {
    vec![
        SelectOption {
            value: "s".to_string(),
            label: "Small".to_string(),
        },
        SelectOption {
            value: "m".to_string(),
            label: "Medium".to_string(),
        },
        SelectOption {
            value: "l".to_string(),
            label: "Large".to_string(),
        },
    ]
}

struct RadioGroupApp;

impl View for RadioGroupApp {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let value = use_state(ctx, String::new());
        let value_val = value.get();
        let value_clone = value.clone();

        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("RadioGroup Test"))
            .child(
                RadioGroup::new(size_options())
                    .label("Size")
                    .value(&value_val)
                    .on_change(move |v: String| {
                        value_clone.set(v);
                    }),
            )
            .child(TextBlock::paragraph(&format!("Value: {}", value_val)))
            .into()
    }
}

struct MultiSelectApp;

impl View for MultiSelectApp {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let values = use_state(ctx, Vec::<String>::new());
        let values_val = values.get();
        let values_clone = values.clone();

        Layout::vertical()
            .gap(16.0)
            .child(TextBlock::h1("MultiSelect Test"))
            .child(
                MultiSelect::new(size_options())
                    .label("Sizes")
                    .placeholder("Pick sizes")
                    .values(values_val.clone())
                    .on_change(move |v: Vec<String>| {
                        values_clone.set(v);
                    }),
            )
            .child(TextBlock::paragraph(&format!(
                "Values: {}",
                values_val.join(",")
            )))
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
        "query" => RustyServer::new(port, || QueryApp),
        "spacer" => RustyServer::new(port, || SpacerApp),
        "separator" => RustyServer::new(port, || SeparatorApp),
        "container" => RustyServer::new(port, || ContainerApp),
        "icon" => RustyServer::new(port, || IconApp),
        "image" => RustyServer::new(port, || ImageApp),
        "avatar" => RustyServer::new(port, || AvatarApp),
        "callout" => RustyServer::new(port, || CalloutApp),
        "skeleton" => RustyServer::new(port, || SkeletonApp),
        "expandable" => RustyServer::new(port, || ExpandableApp),
        "list" => RustyServer::new(port, || ListApp),
        "text_area" => RustyServer::new(port, || TextAreaApp),
        "slider" => RustyServer::new(port, || SliderApp),
        "date_input" => RustyServer::new(port, || DateInputApp),
        "color_input" => RustyServer::new(port, || ColorInputApp),
        "radio_group" => RustyServer::new(port, || RadioGroupApp),
        "multi_select" => RustyServer::new(port, || MultiSelectApp),
        other => {
            eprintln!("Unknown widget: {}", other);
            eprintln!(
                "Known widgets: button, text, text_input, number_input, select, checkbox, \
                 layout, card, query, spacer, separator, container, icon, image, avatar, \
                 callout, skeleton, expandable, list, text_area, slider, date_input, \
                 color_input, radio_group, multi_select"
            );
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
