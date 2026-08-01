#!/bin/sh
# Format staged Rust files with rustfmt.
# Invoked from src/frontend/.husky/pre-commit; must exit 0 on every skip path,
# because husky runs hooks under `sh -e`.
#
# Fully staged files are formatted and re-staged. Partially staged files (staged,
# but with further unstaged worktree edits) are checked as they exist in the index
# and are never rewritten: re-staging one would sweep the unstaged edit into the
# commit without it ever appearing in review.
#
# NUL-delimited throughout, because staged paths may contain spaces.

RS_FILES=$(git diff --cached --name-only -z --diff-filter=ACM | tr '\0' '\n' | grep '\.rs$' || true)

if [ -z "$RS_FILES" ]; then
  echo "No Rust files staged, skipping rustfmt."
  exit 0
fi

if ! command -v rustfmt >/dev/null 2>&1; then
  echo "rustfmt not found on PATH, skipping Rust formatting."
  exit 0
fi

# Staged files that also carry unstaged worktree edits.
PARTIAL=$(printf '%s\n' "$RS_FILES" | tr '\n' '\0' \
  | xargs -0 -r git diff --name-only -z -- | tr '\0' '\n' | sed '/^$/d' || true)
if [ -n "$PARTIAL" ]; then
  FULL=$(printf '%s\n' "$RS_FILES" | grep -vxF "$PARTIAL" || true)
else
  FULL=$RS_FILES
fi

# skip_children=true keeps rustfmt from descending into `mod` declarations and
# rewriting unstaged sibling files.
if [ -n "$FULL" ]; then
  echo "Formatting staged Rust files..."
  printf '%s\n' "$FULL" | tr '\n' '\0' \
    | xargs -0 -r rustfmt --edition 2021 --config skip_children=true
  printf '%s\n' "$FULL" | tr '\n' '\0' | xargs -0 -r git add
fi

if [ -n "$PARTIAL" ]; then
  echo "Checking partially staged Rust files against the index (not rewritten):"
  # rustfmt reading stdin exits 0 whether or not it would reformat, so detect a
  # diff from its output rather than its status. The subshell reports through its
  # exit code because a `while` inside a pipeline cannot export a variable.
  if ! printf '%s\n' "$PARTIAL" | (
    BAD=0
    while IFS= read -r f; do
      [ -n "$f" ] || continue
      if [ -n "$(git show ":$f" | rustfmt --edition 2021 --config skip_children=true --check 2>&1)" ]; then
        echo "  $f"
        BAD=1
      fi
    done
    [ "$BAD" -eq 0 ]
  ); then
    echo "ERROR: a partially staged Rust file is not formatted in the index."
    echo "Run: cargo fmt --all"
    exit 1
  fi
fi
