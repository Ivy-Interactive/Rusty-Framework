#!/bin/sh
# Test harness for src/frontend/.husky/pre-commit.
# Runs from the repo root. Creates throwaway repos under mktemp -d to exercise
# the hook in isolation. Covers 7 branches:
#
#   1. Dirty .rs staged                          -> exit 0; formatted; re-staged
#   2. No .rs staged                             -> exit 0
#   3. Already-clean .rs                         -> exit 0; unchanged
#   4. rustfmt absent from PATH                  -> exit 0; skip message
#   5. Staged path with a space                  -> exit 0; formatted; re-staged
#   6. Unstaged sibling module                   -> staged formatted; unstaged untouched
#   7. Conflict markers staged                   -> exit 1; error message
#
# The harness treats the hook as a black box: it copies the hook and scripts/
# (if present) into a scratch repo, invokes the hook the way .husky/_/h does
# (sh -e <hook> from the repo root), and asserts on the exit code, index state,
# and worktree state. It never greps the hook for a particular command.

set -e

REPO_ROOT="$(pwd)"
HOOK="$REPO_ROOT/src/frontend/.husky/pre-commit"
SCRIPTS_DIR="$REPO_ROOT/scripts"

if [ ! -f "$HOOK" ]; then
  echo "FAIL: $HOOK does not exist"
  exit 1
fi

# Helper: create a throwaway git repo with the hook installed
setup_scratch_repo() {
  SCRATCH="$(mktemp -d)"
  cd "$SCRATCH"
  git init -q
  git config user.name "Test"
  git config user.email "test@test"

  mkdir -p .husky
  cp "$HOOK" .husky/pre-commit
  chmod +x .husky/pre-commit

  # Copy scripts/ if it exists (the delegated helper needs it)
  if [ -d "$SCRIPTS_DIR" ]; then
    cp -r "$SCRIPTS_DIR" ./scripts
  fi

  echo "$SCRATCH"
}

# Helper: run the hook and capture its exit code
run_hook() {
  # Never pipe the invocation - that reports pipe status, not hook status
  OUTPUT_FILE="$(mktemp)"
  set +e
  sh -e .husky/pre-commit > "$OUTPUT_FILE" 2>&1
  EXIT_CODE=$?
  set -e
  cat "$OUTPUT_FILE"
  rm "$OUTPUT_FILE"
  return $EXIT_CODE
}

# Helper: assert equality
assert_eq() {
  local desc="$1"
  local want="$2"
  local got="$3"
  if [ "$want" != "$got" ]; then
    echo "  FAIL: $desc (want [$want] got [$got])"
    return 1
  fi
}

# Helper: assert file content
assert_file_content() {
  local desc="$1"
  local file="$2"
  local want="$3"
  local got="$(cat "$file" 2>/dev/null || echo '<missing>')"
  if [ "$want" != "$got" ]; then
    echo "  FAIL: $desc (want [$want] got [$got])"
    return 1
  fi
}

BRANCH_FAILURES=0

# Branch 1: Dirty .rs staged
echo "Branch 1: dirty .rs staged"
SCRATCH=$(setup_scratch_repo)
printf 'fn  main ( ) {\n}\n' > test.rs
git add test.rs
run_hook
assert_eq "exit 0" "0" "$?" || BRANCH_FAILURES=$((BRANCH_FAILURES + 1))
assert_file_content "worktree reformatted" "test.rs" "fn main() {}" || BRANCH_FAILURES=$((BRANCH_FAILURES + 1))
STAGED_CONTENT="$(git show :test.rs 2>/dev/null || echo '<not staged>')"
assert_eq "index reformatted" "fn main() {}" "$STAGED_CONTENT" || BRANCH_FAILURES=$((BRANCH_FAILURES + 1))
DIFF_OUTPUT="$(git diff --name-only)"
assert_eq "no diff introduced" "" "$DIFF_OUTPUT" || BRANCH_FAILURES=$((BRANCH_FAILURES + 1))
rm -rf "$SCRATCH"
echo "  PASS"

# Branch 2: No .rs staged
echo "Branch 2: no .rs staged"
SCRATCH=$(setup_scratch_repo)
echo "hello" > test.txt
git add test.txt
run_hook
assert_eq "exit 0" "0" "$?" || BRANCH_FAILURES=$((BRANCH_FAILURES + 1))
rm -rf "$SCRATCH"
echo "  PASS"

# Branch 3: Already-clean .rs
echo "Branch 3: already-clean .rs"
SCRATCH=$(setup_scratch_repo)
printf 'fn main() {}\n' > test.rs
git add test.rs
BEFORE_HASH="$(git hash-object test.rs)"
run_hook
assert_eq "exit 0" "0" "$?" || BRANCH_FAILURES=$((BRANCH_FAILURES + 1))
AFTER_HASH="$(git hash-object test.rs)"
assert_eq "index blob unchanged" "$BEFORE_HASH" "$AFTER_HASH" || BRANCH_FAILURES=$((BRANCH_FAILURES + 1))
DIFF_OUTPUT="$(git diff --name-only)"
assert_eq "no diff introduced" "" "$DIFF_OUTPUT" || BRANCH_FAILURES=$((BRANCH_FAILURES + 1))
rm -rf "$SCRATCH"
echo "  PASS"

# Branch 4: rustfmt absent from PATH
echo "Branch 4: rustfmt absent from PATH"
SCRATCH=$(setup_scratch_repo)
printf 'fn  main ( ) {\n}\n' > test.rs
git add test.rs

# Strip PATH to a temp dir but keep git
STRIPPED_PATH="$(dirname "$(command -v git)"):/usr/bin:/bin:$(mktemp -d)"
export PATH="$STRIPPED_PATH"

# Precondition: rustfmt really is absent
if command -v rustfmt >/dev/null 2>&1; then
  echo "  FAIL: precondition failed - rustfmt still on stripped PATH"
  BRANCH_FAILURES=$((BRANCH_FAILURES + 1))
else
  HOOK_OUTPUT="$(run_hook 2>&1)"
  assert_eq "exit 0" "0" "$?" || BRANCH_FAILURES=$((BRANCH_FAILURES + 1))
  if echo "$HOOK_OUTPUT" | grep -q "skipping"; then
    : # skip message present, as expected
  else
    echo "  FAIL: skip message absent"
    BRANCH_FAILURES=$((BRANCH_FAILURES + 1))
  fi
  assert_file_content "file untouched" "test.rs" "fn  main ( ) {
}" || BRANCH_FAILURES=$((BRANCH_FAILURES + 1))
  echo "  PASS"
fi

# Restore PATH
export PATH="$REPO_ROOT/../../../..:$PATH"
rm -rf "$SCRATCH"

# Branch 5: Staged path with a space
echo "Branch 5: staged path containing a space"
SCRATCH=$(setup_scratch_repo)
printf 'fn  main ( ) {\n}\n' > "my file.rs"
git add "my file.rs"
run_hook
assert_eq "exit 0 (a space must not abort the commit)" "0" "$?" || BRANCH_FAILURES=$((BRANCH_FAILURES + 1))
assert_file_content "reformatted" "my file.rs" "fn main() {}" || BRANCH_FAILURES=$((BRANCH_FAILURES + 1))
STAGED_CONTENT="$(git show :"my file.rs" 2>/dev/null || echo '<not staged>')"
assert_eq "re-staged" "fn main() {}" "$STAGED_CONTENT" || BRANCH_FAILURES=$((BRANCH_FAILURES + 1))
rm -rf "$SCRATCH"
echo "  PASS"

# Branch 6: Unstaged sibling module (skip_children)
echo "Branch 6: unstaged sibling module"
SCRATCH=$(setup_scratch_repo)
printf 'pub mod child;\nfn  main ( ) {\n}\n' > root.rs
printf 'pub fn  helper ( ) {\n}\n' > child.rs
git add root.rs child.rs
git commit -q -m "initial"
# Now dirty root.rs and stage it; child.rs stays committed but unstaged
printf 'pub mod child;\nfn  main ( ) {\n  println!("test");\n}\n' > root.rs
git add root.rs
run_hook
assert_eq "exit 0" "0" "$?" || BRANCH_FAILURES=$((BRANCH_FAILURES + 1))
assert_file_content "staged root.rs formatted" "root.rs" 'pub mod child;
fn main() {
    println!("test");
}' || BRANCH_FAILURES=$((BRANCH_FAILURES + 1))
# child.rs must be untouched (still the dirty spacing from commit)
assert_file_content "unstaged child.rs untouched" "child.rs" 'pub fn  helper ( ) {
}' || BRANCH_FAILURES=$((BRANCH_FAILURES + 1))
rm -rf "$SCRATCH"
echo "  PASS"

# Branch 7: Conflict markers staged
echo "Branch 7: conflict markers staged"
SCRATCH=$(setup_scratch_repo)
printf 'fn main() {\n<<<<<<< HEAD\n}\n' > test.rs
git add test.rs
set +e
HOOK_OUTPUT="$(run_hook 2>&1)"
EXIT_CODE=$?
set -e
assert_eq "exit 1" "1" "$EXIT_CODE" || BRANCH_FAILURES=$((BRANCH_FAILURES + 1))
if echo "$HOOK_OUTPUT" | grep -q "conflict"; then
  : # error message present, as expected
else
  echo "  FAIL: conflict error message absent"
  BRANCH_FAILURES=$((BRANCH_FAILURES + 1))
fi
rm -rf "$SCRATCH"
echo "  PASS"

# Summary
if [ "$BRANCH_FAILURES" -eq 0 ]; then
  echo ""
  echo "ALL BRANCHES PASS"
  exit 0
else
  echo ""
  echo "FAILURES: $BRANCH_FAILURES"
  exit 1
fi
