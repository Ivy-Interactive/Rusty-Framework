#!/bin/sh
# Fail when the working tree has lost a test that <base-ref> had.
#
# The four cargo gates cannot see a deleted test: a test module removed wholesale
# takes its own assertions with it, so build/test/clippy/fmt all stay green while
# coverage silently drops. Compare enumerated test names, not a count -- the
# 53dff77 merge deleted 36 tests while the workspace total rose 222 -> 266.
#
# Usage: scripts/check-test-inventory.sh <base-ref>
set -eu

BASE="${1:?usage: check-test-inventory.sh <base-ref>}"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# `cargo test -- --list` prints one "<path>: test" line per test. Anything that is
# not that shape (a compile error, a cargo diagnostic) must abort rather than
# filter down to an empty list, which would read as "no tests were removed".
#
# Doctest names carry the line the ``` block starts on -- "src\lib.rs - foo (line
# 27)" -- so an unrelated edit above one renames it and it would read as deleted
# and added. Verified: inserting two comment lines at the top of
# rusty/src/shared/ivy_node.rs renamed all four of its doctests. Strip the suffix;
# the remaining names stay unique (404 of 404 at the time of writing).
list_tests() {
  out="$1"
  if ! cargo test --workspace -- --list > "$out.raw" 2> "$out.err"; then
    echo "error: 'cargo test --workspace -- --list' failed in $(pwd):" >&2
    cat "$out.err" >&2
    exit 1
  fi
  sed -n 's/: test$//p' "$out.raw" | sed 's/ (line [0-9]*)$//' | LC_ALL=C sort > "$out"
  if [ ! -s "$out" ]; then
    echo "error: enumerated 0 tests in $(pwd) -- refusing to call that a clean run." >&2
    exit 1
  fi
}

# The base tree is extracted, never checked out, so the working tree is untouched.
# It needs its own CARGO_TARGET_DIR: rusty-docs/build.rs writes the gitignored
# src/generated/, and only reruns when the target dir holds no cached fingerprint.
# Sharing a warm target dir makes the base build fail with E0583.
mkdir -p "$WORK/base-tree"
git archive "$BASE" | tar -x -C "$WORK/base-tree"
( cd "$WORK/base-tree" && CARGO_TARGET_DIR="$WORK/base-target" list_tests "$WORK/base" )
list_tests "$WORK/head"

MISSING="$(LC_ALL=C comm -23 "$WORK/base" "$WORK/head")"
if [ -n "$MISSING" ]; then
  echo "error: these tests exist at $BASE but not in the working tree:"
  echo "$MISSING" | sed 's/^/  - /'
  echo
  echo "Deleting a test is allowed, but it must be deliberate: rename, move or drop"
  echo "it on purpose and say so in the commit message. A test module that vanishes"
  echo "while resolving a merge conflict is what this check exists to catch."
  exit 1
fi

echo "Test inventory intact: $(wc -l < "$WORK/base") at $BASE, $(wc -l < "$WORK/head") in the working tree."
