#!/bin/sh
# Format staged Rust files with rustfmt and re-stage the results.
# Invoked from src/frontend/.husky/pre-commit; must exit 0 on every skip path,
# because husky runs hooks under `sh -e`.

RS_ALL=$(mktemp)
RS_SAFE=$(mktemp)
RS_PARTIAL=$(mktemp)
trap 'rm -f "$RS_ALL" "$RS_SAFE" "$RS_PARTIAL"' EXIT

# `|| true`: grep exits 1 when nothing matches, which would abort the commit.
git diff --cached --name-only --diff-filter=ACM | grep '\.rs$' > "$RS_ALL" || true
[ -s "$RS_ALL" ] || { echo "No Rust files staged, skipping rustfmt."; exit 0; }

if ! command -v rustfmt >/dev/null 2>&1; then
  echo "rustfmt not found on PATH, skipping Rust formatting."
  exit 0
fi

# A `while read` loop, not `xargs`: xargs word-splits and exits 123 on `my file.rs`.
: > "$RS_SAFE"
: > "$RS_PARTIAL"
while IFS= read -r f; do
  if [ -n "$(git diff --name-only -- "$f")" ]; then
    printf '%s\n' "$f" >> "$RS_PARTIAL"
  else
    printf '%s\n' "$f" >> "$RS_SAFE"
  fi
done < "$RS_ALL"

STATUS=0

if [ -s "$RS_SAFE" ]; then
  echo "Formatting staged Rust files..."
  while IFS= read -r f; do
    if rustfmt --edition 2021 --config skip_children=true "$f"; then
      git add "$f"
    else
      echo "  rustfmt failed on $f - does it parse?"
      STATUS=1
    fi
  done < "$RS_SAFE"
fi

if [ -s "$RS_PARTIAL" ]; then
  echo "Partially staged Rust files - index copy checked, not rewritten:"
  while IFS= read -r f; do
    t=$(mktemp -d)/staged.rs
    git show ":$f" > "$t"
    if ! rustfmt --edition 2021 --config skip_children=true --check "$t" >/dev/null 2>&1; then
      echo "  $f is staged with unstaged edits and its staged copy is not formatted."
      echo "  Run: cargo fmt --all, then stage the result."
      STATUS=1
    fi
    rm -rf "$(dirname "$t")"
  done < "$RS_PARTIAL"
fi

exit $STATUS
