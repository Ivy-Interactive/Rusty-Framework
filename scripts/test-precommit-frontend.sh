#!/bin/sh
# Test harness for the frontend branch of src/frontend/.husky/pre-commit.
# Runs from the repo root, inside the real checkout: lint-staged resolves its
# config from src/frontend/package.json and needs the installed workspace, so
# unlike the Rust cases this cannot use a throwaway `git init` repo.
#
# No `set -e`: a harness must survive a failing assertion and report all of
# them. (`set -e` is also not function-scoped, so toggling it inside a helper
# silently re-arms it for the caller.)

HOOK="src/frontend/.husky/pre-commit"
PROBE="src/frontend/src/__precommit_probe__.ts"
FAILURES=0

fail() {
  echo "FAIL: $1"
  FAILURES=$((FAILURES + 1))
}

# Preconditions - each would otherwise make a case pass or fail vacuously.
[ -f "$HOOK" ] || { echo "FAIL: $HOOK does not exist"; exit 1; }
[ -d src/frontend/node_modules ] || { echo "FAIL: src/frontend/node_modules missing - run pnpm install --frozen-lockfile first"; exit 1; }
[ -z "$(git status --porcelain)" ] || { echo "FAIL: working tree is dirty; refusing to stage probes into it"; exit 1; }

reset_tree() {
  git reset -q >/dev/null 2>&1
  rm -f "$PROBE" __precommit_probe__.txt
  git checkout -q -- . >/dev/null 2>&1
}
trap reset_tree EXIT

# Emulate husky's _/h, which prepends a RELATIVE `node_modules/.bin` to PATH.
# It resolves against cwd, so `vp` is only findable after the hook cd's into
# src/frontend. Without this the hook dies `vp: command not found`, exit 127.
run_hook() {
  OUT_FILE="$(mktemp)"
  ( export PATH="node_modules/.bin:$PATH"; sh -e "$HOOK" ) >"$OUT_FILE" 2>&1
  RC=$?
  OUT="$(cat "$OUT_FILE")"
  rm -f "$OUT_FILE"
}

# Case A: clean staged frontend file -> lint-staged runs, exit 0
printf 'export const probe = 1;\n' > "$PROBE"
git add "$PROBE"
run_hook
[ "$RC" = "0" ] || fail "A: exit $RC (want 0)"
case "$OUT" in *"Vite+ linter"*) ;; *) fail "A: frontend branch did not run" ;; esac
case "$OUT" in *"vp lint --fix"*) ;; *) fail "A: lint-staged did not reach vp lint --fix" ;; esac
reset_tree

# Case B: unparseable staged frontend file -> non-zero, commit blocked
printf 'const broken: = ;;;\n' > "$PROBE"
git add "$PROBE"
run_hook
[ "$RC" != "0" ] || fail "B: exit 0 (an unparseable staged file must block the commit)"
case "$OUT" in *"FAILED"*) ;; *) fail "B: no task failure reported" ;; esac
reset_tree

# Case C: no frontend file staged -> skip message, exit 0
printf 'probe\n' > __precommit_probe__.txt
git add __precommit_probe__.txt
run_hook
[ "$RC" = "0" ] || fail "C: exit $RC (want 0)"
case "$OUT" in *"No frontend files staged"*) ;; *) fail "C: skip message absent" ;; esac
reset_tree

# Case D: node absent from PATH -> exit 1 with an actionable message.
# PATH must keep git (the hook's first command) but drop node. `vp` is a native
# binary and would still resolve, so this asserts the guard, not vp's absence.
NODELESS_PATH="$(dirname "$(command -v git)"):/usr/bin:/bin"
printf 'export const probe = 1;
' > "$PROBE"
git add "$PROBE"
if PATH="$NODELESS_PATH" command -v node >/dev/null 2>&1; then
  fail "D: precondition - node still on the stripped PATH, case would pass vacuously"
else
  OUT_FILE="$(mktemp)"
  ( PATH="node_modules/.bin:$NODELESS_PATH"; export PATH; sh -e "$HOOK" ) >"$OUT_FILE" 2>&1
  RC=$?
  OUT="$(cat "$OUT_FILE")"
  rm -f "$OUT_FILE"
  [ "$RC" = "1" ] || fail "D: exit $RC (want 1 - a missing node must block, not pass)"
  case "$OUT" in *"node is not on PATH"*) ;; *) fail "D: no actionable node message" ;; esac
fi
reset_tree

if [ "$FAILURES" -eq 0 ]; then
  echo "ALL FRONTEND CASES PASS"
  exit 0
fi
echo "FAILURES: $FAILURES"
exit 1
