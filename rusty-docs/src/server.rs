use rusty::prelude::*;

use crate::generated::all_pages;

/// Wrapper view that renders a specific page by index.
struct PageView {
    page_index: usize,
}

impl View for PageView {
    fn build(&self, _ctx: &mut BuildContext) -> Element {
        let pages = all_pages();
        if let Some(page) = pages.into_iter().nth(self.page_index) {
            let view = (page.view_factory)();
            view.build(_ctx)
        } else {
            Element::Empty
        }
    }
}

pub struct DocsShellView;

impl View for DocsShellView {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let pages = all_pages();
        let active_index = use_state(ctx, 0usize);

        // Build sidebar navigation
        let mut sidebar = Layout::vertical().gap(4.0).padding(16.0);
        let mut current_section = String::new();

        for (i, page) in pages.iter().enumerate() {
            if page.section != current_section {
                current_section = page.section.to_string();
                sidebar = sidebar.child(
                    TextBlock::new(&current_section)
                        .bold()
                        .color(Color::Named(NamedColor::Muted)),
                );
            }

            let active = active_index.clone();
            let is_active = active_index.get() == i;

            let btn = if is_active {
                Button::new(page.title)
                    .variant(rusty::widgets::button::ButtonVariant::Ghost)
                    .color(Color::Named(NamedColor::Primary))
                    .on_click(move || {
                        active.set(i);
                    })
            } else {
                Button::new(page.title)
                    .variant(rusty::widgets::button::ButtonVariant::Ghost)
                    .on_click(move || {
                        active.set(i);
                    })
            };

            sidebar = sidebar.child(btn);
        }

        // Build content area using child_view
        let page_view = PageView {
            page_index: active_index.get(),
        };
        let (content, _view_id, _hook_store) = ctx.child_view(page_view, None);

        let content_area = Layout::vertical().padding(32.0).gap(16.0).child(content);

        // Shell layout: sidebar + content
        Layout::horizontal()
            .child(sidebar)
            .child(content_area)
            .into()
    }
}
