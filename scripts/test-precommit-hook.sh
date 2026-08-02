#!/bin/sh
# Black-box tests for the pre-commit Rust formatting step.
# Run from the repo root: sh scripts/test-precommit-hook.sh
#
# Each case builds a throwaway git repo, installs the real hook and scripts/,
# and invokes the hook the way .husky/_/h does (`sh -e <hook>` from the repo
# root of that throwaway repo). Asserts on exit code, index and worktree.

REPO_ROOT=$(pwd)
HOOK="$REPO_ROOT/src/frontend/.husky/pre-commit"
SCRIPTS="$REPO_ROOT/scripts"
FAILURES=0

[ -f "$HOOK" ] || { echo "FAIL: $HOOK does not exist"; exit 1; }

# setup <name>: cd into a fresh scratch repo with the hook installed.
# NOT a command substitution - `cd` inside $( ) runs in a subshell and would
# leave the caller in the real repo, staging test files into it.
setup() {
  SCRATCH=$(mktemp -d)
  cd "$SCRATCH" || exit 1
  git init -q .
  git config user.name t
  git config user.email t@t
  cp "$HOOK" ./pre-commit
  [ -d "$SCRIPTS" ] && cp -r "$SCRIPTS" ./scripts
  CASE="$1"
}

# run: invoke the hook, capture exit code in RC and output in OUT.
run() {
  OUT=$(sh -e ./pre-commit 2>&1)
  RC=$?
}

check() {
  if [ "$2" != "$3" ]; then
    echo "  FAIL [$CASE] $1: want [$2] got [$3]"
    FAILURES=$((FAILURES + 1))
  fi
}

teardown() { cd "$REPO_ROOT" || exit 1; rm -rf "$SCRATCH"; }

echo "1. fully staged dirty .rs -> formatted, re-staged, exit 0"
setup "dirty"
printf 'fn  main ( ) {\n}\n' > a.rs; git add a.rs
run
check "exit" 0 "$RC"
check "index formatted" "fn main() {}" "$(git show :a.rs)"
check "no unstaged residue" "" "$(git diff --name-only)"
teardown

echo "2. staged path containing a space -> formatted, exit 0 (not xargs 123)"
setup "space"
printf 'fn  main ( ) {\n}\n' > "my file.rs"; git add "my file.rs"
run
check "exit" 0 "$RC"
check "index formatted" "fn main() {}" "$(git show ':my file.rs')"
teardown

echo "3. partially staged + unformatted -> blocks, index untouched"
setup "partial-dirty"
printf 'pub fn  a ( ) {}\n' > d.rs; git add d.rs
printf 'pub fn  a ( ) {}\npub fn unreviewed() {}\n' > d.rs
run
check "exit" 1 "$RC"
check "index NOT swept" "pub fn  a ( ) {}" "$(git show :d.rs)"
check "worktree edit preserved" "pub fn  a ( ) {}
pub fn unreviewed() {}" "$(cat d.rs)"
teardown

echo "4. partially staged but already formatted -> passes, exit 0"
setup "partial-clean"
printf 'pub fn a() {}\n' > d.rs; git add d.rs
printf 'pub fn a() {}\npub fn b() {}\n' > d.rs
run
check "exit" 0 "$RC"
check "index untouched" "pub fn a() {}" "$(git show :d.rs)"
teardown

echo "5. no .rs staged -> exit 0 (grep no-match must not abort)"
setup "no-rs"
printf 'x\n' > a.txt; git add a.txt
run
check "exit" 0 "$RC"
teardown

echo "6. already-clean .rs -> idempotent, exit 0"
setup "clean"
printf 'fn main() {}\n' > a.rs; git add a.rs
BEFORE=$(git hash-object a.rs)
run
check "exit" 0 "$RC"
check "blob unchanged" "$BEFORE" "$(git hash-object a.rs)"
teardown

echo "7. non-parsing .rs -> commit blocked (any non-zero)"
setup "nonparsing"
printf 'fn main() { \n' > bad.rs; git add bad.rs
run
# Assert only "blocked", not a specific code: rustfmt's own exit is 123, while an
# implementation that checks per-file and names it returns 1. Both block the commit.
if [ "$RC" -eq 0 ]; then
  echo "  FAIL [$CASE] non-parsing file did not block the commit (exit 0)"
  FAILURES=$((FAILURES + 1))
fi
teardown

echo "8. rustfmt absent from PATH -> skip message, exit 0"
setup "no-rustfmt"
printf 'fn  main ( ) {\n}\n' > a.rs; git add a.rs
# Keep git and sh reachable; drop everything else (notably ~/.cargo/bin).
MINPATH="$(dirname "$(command -v git)"):$(dirname "$(command -v sh)")"
if env PATH="$MINPATH" sh -c 'command -v rustfmt >/dev/null 2>&1'; then
  echo "  FAIL [$CASE] precondition: rustfmt still on stripped PATH"
  FAILURES=$((FAILURES + 1))
else
  OUT=$(env PATH="$MINPATH" sh -e ./pre-commit 2>&1); RC=$?
  check "exit" 0 "$RC"
  case "$OUT" in *"rustfmt not found"*) ;; *) echo "  FAIL [$CASE] no skip message"; FAILURES=$((FAILURES + 1));; esac
fi
teardown

echo "9. unstaged sibling module -> skip_children keeps it untouched"
setup "skip-children"
printf 'mod child;\nfn main() {\n    child::c();\n}\n' > root.rs
printf 'pub fn c() {}\n' > child.rs
git add root.rs child.rs; git commit -qm base
printf 'mod child;\nfn  main ( ) {\n    child::c();\n}\n' > root.rs; git add root.rs
printf 'pub fn  c ( ) {}\n' > child.rs
run
check "exit" 0 "$RC"
check "sibling untouched" "pub fn  c ( ) {}" "$(cat child.rs)"
check "sibling not staged" "" "$(git diff --cached --name-only -- child.rs)"
teardown

echo "10. deleted staged .rs -> exit 0 (ACM filter excludes it)"
setup "deleted"
printf 'fn main() {}\n' > a.rs; git add a.rs; git commit -qm base
git rm -q a.rs
run
check "exit" 0 "$RC"
teardown

echo "11. conflict markers staged -> exit 1 (pre-existing check still works)"
setup "conflict"
printf 'fn main() {\n<<<<<<< HEAD\n}\n' > a.rs; git add a.rs
run
check "exit" 1 "$RC"
case "$OUT" in *conflict*) ;; *) echo "  FAIL [$CASE] no conflict message"; FAILURES=$((FAILURES + 1));; esac
teardown

echo "12. scripts/ absent (checkout predating the script) -> exit 0, not 127"
# `setup` always copies scripts/, so this case removes it again: a checkout that
# predates c5121ee has the hook but no script, and an unguarded `sh ./scripts/...`
# makes every commit there fail with 127. 123 commits are in that range.
setup "no-script"
rm -rf ./scripts
printf 'x\n' > a.txt; git add a.txt
run
check "exit" 0 "$RC"
case "$OUT" in *"No such file"*) echo "  FAIL [$CASE] unguarded call: $OUT"; FAILURES=$((FAILURES + 1));; esac
teardown

echo "13. script failure blocks even without sh -e (|| exit 1 propagates)"
# `run` uses `sh -e`, under which a bare call already propagates. A developer
# invoking `sh .husky/pre-commit` by hand gets no -e, and there the exit status
# of a non-final command is discarded unless the call says `|| exit 1`.
setup "propagate-no-e"
printf 'pub fn  a ( ) {}\n' > d.rs; git add d.rs
printf 'pub fn  a ( ) {}\npub fn unreviewed() {}\n' > d.rs
OUT=$(sh ./pre-commit 2>&1); RC=$?
check "exit" 1 "$RC"
check "index NOT swept" "pub fn  a ( ) {}" "$(git show :d.rs)"
teardown

echo ""
if [ "$FAILURES" -eq 0 ]; then
  echo "ALL 13 CASES PASS"
  exit 0
fi
echo "FAILURES: $FAILURES"
exit 1
