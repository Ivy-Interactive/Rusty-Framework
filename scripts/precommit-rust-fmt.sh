#!/bin/sh
# Format staged Rust files with rustfmt and re-stage the results.
# Invoked from src/frontend/.husky/pre-commit; must exit 0 on every skip path,
# because husky runs hooks under `sh -e`.

if ! git diff --cached --name-only -z --diff-filter=ACM | grep -qz '\.rs$'; then
  echo "No Rust files staged, skipping rustfmt."
  exit 0
fi

if ! command -v rustfmt >/dev/null 2>&1; then
  echo "rustfmt not found on PATH, skipping Rust formatting."
  exit 0
fi

# skip_children=true keeps rustfmt from descending into `mod` declarations and
# rewriting unstaged sibling files.
echo "Formatting staged Rust files..."
git diff --cached --name-only -z --diff-filter=ACM \
  | grep -z '\.rs$' \
  | xargs -0 rustfmt --edition 2021 --config skip_children=true
git diff --cached --name-only -z --diff-filter=ACM \
  | grep -z '\.rs$' \
  | xargs -0 git add
