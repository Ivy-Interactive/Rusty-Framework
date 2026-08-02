#!/bin/sh
# Run the compile-class Rust gates on a commit that records a merge.
#
# The rustfmt block in .husky/pre-commit cannot see these failures: a file that
# does not parse, a symbol the merge deleted, and a clippy regression are all
# exit 0 to `rustfmt --check`. A merge resolution is the one commit whose exact
# content no branch ever compiled, so it is the one that needs a real build.
#
# Called with `--merge` from .husky/pre-merge-commit (a clean auto-merge, where
# MERGE_HEAD does NOT yet exist), and with no argument from .husky/pre-commit
# (where MERGE_HEAD is present only while concluding a conflicted merge).

IS_MERGE=0
[ "$1" = "--merge" ] && IS_MERGE=1
[ -f "$(git rev-parse --git-dir)/MERGE_HEAD" ] && IS_MERGE=1
[ -n "${RUSTY_FORCE_GATES-}" ] && IS_MERGE=1

if [ "$IS_MERGE" -eq 0 ]; then
  exit 0
fi

if ! git diff --cached --name-only -z --diff-filter=ACMR | tr '\0' '\n' | grep -q '\.rs$'; then
  echo "Merge touches no Rust files, skipping cargo gates."
  exit 0
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found on PATH, skipping Rust gates."
  exit 0
fi

STATUS=0

echo "Merge with Rust changes - running cargo check --workspace..."
if ! cargo check --workspace --quiet; then
  echo ""
  echo "ERROR: 'cargo check --workspace' failed on this merge."
  STATUS=1
fi

if [ "$STATUS" -eq 0 ]; then
  if command -v cargo-clippy >/dev/null 2>&1; then
    echo "Running cargo clippy --workspace --all-targets..."
    if ! cargo clippy --workspace --all-targets --quiet -- -D warnings; then
      echo ""
      echo "ERROR: 'cargo clippy --workspace --all-targets' failed on this merge."
      STATUS=1
    fi
  else
    echo "cargo-clippy not found on PATH, skipping clippy."
  fi
fi

if [ "$STATUS" -ne 0 ]; then
  echo "Fix the merge result, or bypass with 'git commit --no-verify'."
fi

exit "$STATUS"
