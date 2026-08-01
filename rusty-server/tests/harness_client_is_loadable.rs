//! `e2e/app/index.html` is the widgets' only client and is served by the
//! `widget_harness` binary in this crate. It is in no browser-based gate, and
//! hand-resolved merge conflicts have twice left it unloadable on `main` while
//! every Rust gate stayed green. These invariants are the cheap half of that
//! check: they need no browser, and they catch the duplicated-`<script>` case
//! that a `pageerror` probe reports as clean.

use std::path::{Path, PathBuf};

fn client_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("e2e/app/index.html")
}

fn script_body(html: &str) -> &str {
    let open_end = html.find("<script>").expect("no <script> tag") + "<script>".len();
    let close = html[open_end..]
        .find("</script>")
        .expect("unterminated <script>")
        + open_end;
    &html[open_end..close]
}

#[test]
fn harness_client_script_is_well_formed() {
    let path = client_path();
    let html = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

    let opens = html.matches("<script").count();
    let closes = html.matches("</script>").count();
    assert_eq!(
        (opens, closes),
        (1, 1),
        "{} has {opens} <script> and {closes} </script>: the browser ends the script at the FIRST \
         close tag, so everything after it renders as visible text and its case arms go dead",
        path.display()
    );

    let body = script_body(&html);
    let stripped: String = strip_comments(body);
    let opens_b = stripped.matches('{').count() as i64;
    let closes_b = stripped.matches('}').count() as i64;
    assert_eq!(
        opens_b - closes_b,
        0,
        "{} script has a brace imbalance of {:+}: a hand-resolved merge conflict left a block \
         unclosed or over-closed",
        path.display(),
        opens_b - closes_b
    );

    let mut labels: Vec<&str> = Vec::new();
    let mut rest = body;
    while let Some(i) = rest.find("case '") {
        rest = &rest[i + 6..];
        if let Some(j) = rest.find("':") {
            labels.push(&rest[..j]);
        }
    }
    let mut dups: Vec<&str> = labels
        .iter()
        .filter(|l| labels.iter().filter(|x| x == l).count() > 1)
        .copied()
        .collect();
    dups.sort_unstable();
    dups.dedup();
    assert!(
        dups.is_empty(),
        "{} has duplicate case labels ({}): a merge conflict duplicated a block of arms",
        path.display(),
        dups.join(", ")
    );
}

fn strip_comments(src: &str) -> String {
    let b = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'/' && i + 1 < b.len() && b[i + 1] == b'/' {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else if b[i] == b'/' && i + 1 < b.len() && b[i + 1] == b'*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(b.len());
        } else {
            out.push(b[i] as char);
            i += 1;
        }
    }
    out
}
