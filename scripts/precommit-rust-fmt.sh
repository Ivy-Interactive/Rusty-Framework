#!/bin/sh
# Format staged Rust files with rustfmt and re-stage the results.
# Invoked from src/frontend/.husky/pre-commit; must exit 0 on every skip path,
# because husky runs hooks under `sh -e`.

RS_ALL=$(mktemp)
RS_FULL=$(mktemp)
RS_PARTIAL=$(mktemp)
trap 'rm -f "$RS_ALL" "$RS_FULL" "$RS_PARTIAL"' EXIT

# `|| true`: grep exits 1 when nothing matches, which would abort the commit.
git diff --cached --name-only --diff-filter=ACM | grep '\.rs$' > "$RS_ALL" || true
[ -s "$RS_ALL" ] || { echo "No Rust files staged, skipping rustfmt."; exit 0; }

if ! command -v rustfmt >/dev/null 2>&1; then
  echo "rustfmt not found on PATH, skipping Rust formatting."
  exit 0
fi

# A `while read` loop, not `xargs`: xargs word-splits on spaces and exits 123
# on a staged path like `my file.rs`.
: > "$RS_FULL"
: > "$RS_PARTIAL"
while IFS= read -r f; do
  if [ -n "$(git diff --name-only -- "$f")" ]; then
    printf '%s\n' "$f" >> "$RS_PARTIAL"
  else
    printf '%s\n' "$f" >> "$RS_FULL"
  fi
done < "$RS_ALL"

STATUS=0

if [ -s "$RS_FULL" ]; then
  echo "Formatting staged Rust files..."
  while IFS= read -r f; do
    # skip_children=true keeps rustfmt from descending into `mod` declarations
    # and rewriting unstaged sibling files.
    if rustfmt --edition 2021 --config skip_children=true "$f"; then
      git add "$f"
    else
      echo "  rustfmt failed on $f - does it parse?"
      STATUS=1
    fi
  done < "$RS_FULL"
fi

if [ -s "$RS_PARTIAL" ]; then
  echo "Partially staged Rust files - checked against the index, not rewritten:"
  IDXDIR=$(mktemp -d)
  while IFS= read -r f; do
    # rustfmt --check needs a real path: on stdin it always exits 0. Extract the
    # INDEX copy, so the check sees what is actually being committed.
    IDX="$IDXDIR/idx.rs"
    git show ":$f" > "$IDX" 2>/dev/null || continue
    if ! rustfmt --edition 2021 --config skip_children=true --check "$IDX" >/dev/null 2>&1; then
      echo "  $f is staged with unstaged edits and its index copy is not formatted."
      echo "  Run: cargo fmt --all"
      STATUS=1
    fi
  done < "$RS_PARTIAL"
  rm -rf "$IDXDIR"
fi

exit $STATUS
