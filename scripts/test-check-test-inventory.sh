#!/bin/sh
# Black-box tests for the test-inventory gate.
# Run from the repo root: sh scripts/test-check-test-inventory.sh
#
# Each case builds a throwaway cargo workspace, commits a base revision, mutates
# the working tree, and runs the real scripts/check-test-inventory.sh against
# that base. Asserts on exit code and on which names the gate reports.
#
# A synthetic workspace, not this repo: the gate compiles the base tree once, so
# running it against Rusty-Framework costs ~45s per case. The probe crate takes
# under a second and exercises the same three code paths that matter -- named
# unit tests, a doctest whose name carries its line number, and a base tree that
# does not compile.

REPO_ROOT=$(pwd)
GATE="$REPO_ROOT/scripts/check-test-inventory.sh"
FAILURES=0

[ -f "$GATE" ] || { echo "FAIL: $GATE does not exist"; exit 1; }

# setup <name>: cd into a fresh scratch workspace with one committed revision.
# NOT a command substitution - `cd` inside $( ) runs in a subshell and would
# leave the caller in the real repo.
setup() {
  SCRATCH=$(mktemp -d)
  cd "$SCRATCH" || exit 1
  CASE="$1"
  cat > Cargo.toml <<'EOF'
[package]
name = "inv-probe"
version = "0.1.0"
edition = "2021"
EOF
  mkdir -p src
  cat > src/lib.rs <<'EOF'
/// Adds two numbers.
///
/// ```
/// assert_eq!(inv_probe::add(1, 1), 2);
/// ```
pub fn add(a: u32, b: u32) -> u32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::add;

    #[test]
    fn alpha() {
        assert_eq!(add(1, 1), 2);
    }

    #[test]
    fn beta() {
        assert_eq!(add(2, 2), 4);
    }

    #[test]
    fn gamma() {
        assert_eq!(add(3, 3), 6);
    }
}
EOF
  git init -q .
  git config user.name t
  git config user.email t@t
  # The probe files are written with LF; without this git warns on every add.
  git config core.autocrlf false
  git add -A
  git commit -qm base
  BASE=$(git rev-parse HEAD)
}

# run: invoke the gate against the committed base, capture exit code and output.
run() {
  OUT=$(sh "$GATE" "$BASE" 2>&1)
  RC=$?
}

check() {
  if [ "$2" != "$3" ]; then
    echo "  FAIL [$CASE] $1: want [$2] got [$3]"
    FAILURES=$((FAILURES + 1))
  fi
}

# reports <name>: the gate's missing-list must contain <name>.
reports() {
  case "$OUT" in
    *"  - $1"*) ;;
    *) echo "  FAIL [$CASE] gate did not report missing test '$1'"; FAILURES=$((FAILURES + 1));;
  esac
}

# silent_about <name>: the gate must not list <name> as missing.
silent_about() {
  case "$OUT" in
    *"  - $1"*) echo "  FAIL [$CASE] gate wrongly reported '$1' as missing"; FAILURES=$((FAILURES + 1));;
    *) ;;
  esac
}

teardown() { cd "$REPO_ROOT" || exit 1; rm -rf "$SCRATCH"; }

echo "1. unmodified tree -> exit 0, reports the inventory as intact"
setup "clean"
run
check "exit" 0 "$RC"
case "$OUT" in *"Test inventory intact"*) ;; *) echo "  FAIL [$CASE] no intact message: $OUT"; FAILURES=$((FAILURES + 1));; esac
teardown

echo "2. whole #[cfg(test)] module deleted -> exit 1, lists all 3 names"
setup "module-deleted"
# The shape of 53dff77: the module vanishes while resolving a conflict. Every
# cargo gate stays green because the deleted code was the only thing asserting.
sed '/#\[cfg(test)\]/,$d' src/lib.rs > src/lib.rs.new && mv src/lib.rs.new src/lib.rs
cargo test --workspace --quiet > /dev/null 2>&1
check "cargo test still green on the mutated tree" 0 "$?"
run
check "exit" 1 "$RC"
reports "tests::alpha"
reports "tests::beta"
reports "tests::gamma"
teardown

echo "3. 3 renamed away + 5 added (net total UP by 2) -> exit 1, lists the 3"
setup "net-increase"
# A count-based gate passes this. 53dff77 moved the workspace total 222 -> 266
# while deleting 36 tests, which is exactly why this gate compares names.
cat > src/lib.rs <<'EOF'
/// Adds two numbers.
///
/// ```
/// assert_eq!(inv_probe::add(1, 1), 2);
/// ```
pub fn add(a: u32, b: u32) -> u32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::add;

    #[test]
    fn one() {
        assert_eq!(add(1, 0), 1);
    }

    #[test]
    fn two() {
        assert_eq!(add(1, 1), 2);
    }

    #[test]
    fn three() {
        assert_eq!(add(1, 2), 3);
    }

    #[test]
    fn four() {
        assert_eq!(add(2, 2), 4);
    }

    #[test]
    fn five() {
        assert_eq!(add(2, 3), 5);
    }
}
EOF
run
check "exit" 1 "$RC"
reports "tests::alpha"
reports "tests::beta"
reports "tests::gamma"
teardown

echo "4. base tree does not compile -> exit 1 with cargo's error, never exit 0"
setup "broken-base"
# Committing a base that does not build, then fixing the working tree: without
# propagating cargo's exit status the base enumerates 0 tests, comm reports
# nothing missing, and the gate exits 0 on a tree it never inspected.
printf 'this is not rust\n' > src/lib.rs
git add -A
git commit -qm "broken base"
BASE=$(git rev-parse HEAD)
git checkout -q HEAD~1 -- src/lib.rs
run
check "exit" 1 "$RC"
case "$OUT" in *"--list' failed"*) ;; *) echo "  FAIL [$CASE] no cargo failure message: $OUT"; FAILURES=$((FAILURES + 1));; esac
teardown

echo "5. lines inserted above a doctest -> exit 0 (line-number suffix stripped)"
setup "doctest-shift"
# Doctests are named "src\lib.rs - add (line 3)". An unrelated edit above one
# renames it, so a raw-name comparison reads a comment as a deleted test.
# Verified against the real repo: two comment lines at the top of
# rusty/src/shared/ivy_node.rs renamed all four of its doctests.
printf '// shifted\n// shifted\n' > src/lib.rs.new
cat src/lib.rs >> src/lib.rs.new
mv src/lib.rs.new src/lib.rs
grep -q "line 5" "$(cargo test --workspace -- --list 2>/dev/null > /tmp/inv-probe-list; echo /tmp/inv-probe-list)" \
  || { echo "  FAIL [$CASE] precondition: doctest did not shift to line 5"; FAILURES=$((FAILURES + 1)); }
rm -f /tmp/inv-probe-list
run
check "exit" 0 "$RC"
silent_about "src\\lib.rs - add (line 3)"
teardown

echo "6. a doctest actually removed -> exit 1 and names it"
setup "doctest-removed"
# The other half of case 5: stripping the line suffix must not make the gate
# blind to a doctest that is genuinely gone.
sed '/^\/\/\/ ```$/,/^\/\/\/ ```$/d' src/lib.rs > src/lib.rs.new && mv src/lib.rs.new src/lib.rs
run
check "exit" 1 "$RC"
reports "src\\lib.rs - add"
teardown

echo ""
if [ "$FAILURES" -eq 0 ]; then
  echo "ALL 6 CASES PASS"
  exit 0
fi
echo "FAILURES: $FAILURES"
exit 1
