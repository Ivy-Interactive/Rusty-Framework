//! `build.rs` writes Rust source into `src/generated/`, and `cargo fmt --all`
//! reads it back. If the generator's output is not already rustfmt-clean, the
//! CI format check fails on files nobody can edit by hand — so assert the shape
//! here, next to the generator that produces it.

use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn generated_sources_are_rustfmt_clean() {
    if Command::new("rustfmt").arg("--version").output().is_err() {
        eprintln!("rustfmt is not on PATH; skipping");
        return;
    }

    let generated = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated");
    let mut sources = Vec::new();
    collect_rs_files(&generated, &mut sources);

    assert!(
        !sources.is_empty(),
        "no generated sources under {} — did build.rs run?",
        generated.display()
    );
    sources.sort();

    let output = Command::new("rustfmt")
        .args(["--edition", "2021", "--check"])
        .args(&sources)
        .output()
        .expect("rustfmt --check should run");

    let diff = String::from_utf8_lossy(&output.stdout);
    assert!(
        diff.is_empty(),
        "build.rs emitted code rustfmt would reformat:\n{}",
        diff
    );
    assert!(
        output.status.success(),
        "rustfmt exited with {}:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", dir.display(), e));

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}
