//! The HTML client, compiled into the executable.
//!
//! A desktop binary must not need a directory of files next to it, so the renderer is
//! embedded rather than served from disk via `RustyServer::with_static_dir`.

/// The Rusty renderer, embedded at compile time.
pub const INDEX_HTML: &str = include_str!("../assets/index.html");

/// The embedded renderer.
pub fn index_html() -> &'static str {
    INDEX_HTML
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_html_is_a_real_document() {
        assert!(!INDEX_HTML.trim().is_empty());
        assert!(INDEX_HTML.contains("<!DOCTYPE html>"));
        assert!(INDEX_HTML.contains("new WebSocket"));
        assert!(index_html().len() > 1000, "renderer looks truncated");
    }

    #[test]
    fn index_html_derives_the_socket_host_from_the_page() {
        // `assets/index.html` is a copy of `e2e/app/index.html` with the harness's
        // `?port=` override removed. If someone re-copies the e2e file wholesale, the
        // shell silently starts honouring a query param it never sets — and on a
        // page served without one, `pageQuery.get('port')` is null and the socket URL
        // falls back correctly, so the bug would not show up at runtime.
        assert!(
            !INDEX_HTML.contains("get('port')"),
            "the e2e-only ?port= override must not be in the desktop copy"
        );
        assert!(INDEX_HTML.contains("ws://${window.location.host}/ws"));
    }

    #[test]
    fn index_html_script_is_well_formed() {
        // The renderer is one inline <script>; a duplicated closing tag from a bad merge
        // throws no page error, it just silently truncates the script.
        assert_eq!(INDEX_HTML.matches("<script").count(), 1);
        assert_eq!(INDEX_HTML.matches("</script>").count(), 1);
    }
}
