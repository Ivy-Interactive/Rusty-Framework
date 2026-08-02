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
        // throws no page error, it just silently truncates the script — the browser ends
        // the script at the FIRST close tag and every case arm after it goes dead. The
        // same invariants guard the e2e original in
        // `rusty-server/tests/harness_client_is_loadable.rs`; this copy needs its own
        // check because nothing else loads it in a browser.
        assert_eq!(INDEX_HTML.matches("<script").count(), 1);
        assert_eq!(INDEX_HTML.matches("</script>").count(), 1);
    }

    #[test]
    fn index_html_switch_has_no_duplicate_case_labels() {
        // A merge conflict resolved by keeping both sides duplicates a block of arms; the
        // second copy is unreachable, so a widget silently stops rendering.
        let body = script_body(INDEX_HTML);
        let mut labels: Vec<&str> = Vec::new();
        let mut rest = body;
        while let Some(index) = rest.find("case '") {
            rest = &rest[index + "case '".len()..];
            if let Some(end) = rest.find("':") {
                labels.push(&rest[..end]);
            }
        }
        assert!(!labels.is_empty(), "no case labels found — bad parse");

        let mut duplicates: Vec<&str> = labels
            .iter()
            .filter(|label| labels.iter().filter(|other| other == label).count() > 1)
            .copied()
            .collect();
        duplicates.sort_unstable();
        duplicates.dedup();
        assert!(
            duplicates.is_empty(),
            "duplicate case labels: {}",
            duplicates.join(", ")
        );
    }

    /// The contents of the single inline `<script>`.
    fn script_body(html: &str) -> &str {
        let open_end = html.find("<script>").expect("no <script> tag") + "<script>".len();
        let close = html[open_end..]
            .find("</script>")
            .expect("unterminated <script>")
            + open_end;
        &html[open_end..close]
    }
}
