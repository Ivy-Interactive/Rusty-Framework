# Rusty-Framework Agent Instructions

## Verify

Rusty-Framework uses a hybrid Rust/Node frontend stack. Always verify both sides.

Rust, from the repo root:

```sh
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Frontend, from `src/frontend` (there is no root `package.json`):

```sh
pnpm install --frozen-lockfile
pnpm lint
pnpm exec tsc -b
pnpm run build
pnpm run check:bundle
pnpm test
pnpm format:check
```

**Note:** Rust formatting is enforced on pre-commit via `.husky/pre-commit`. Staged `.rs` files are auto-formatted with `rustfmt --edition 2021 --config skip_children=true` and re-added. Partially staged files are checked as they exist in the index, never rewritten, and will block the commit with "Run: cargo fmt --all" if unformatted.

## Hook rules

Hook state lives in slots keyed by **call index** (`BuildContext::next_hook_index`,
`rusty/src/views/view.rs`), the same ordering rule as React and Ivy. Two ways to break it are
invisible to the compiler, and `rusty-macros` now makes both a compile error — opt in per impl block
with `#[rusty::view]`:

```rust
#[rusty::view]
impl View for MyApp {
    fn build(&self, ctx: &mut BuildContext) -> Element { /* ... */ }
}
```

1. **`conditional_hooks`** — a hook called inside an `if` branch, a `match` arm, a `for`/`while`/
   `loop` body, a closure or an `async` block shifts every later hook's slot, so
   `HookStore::get_or_init_state` starts returning a different hook's value or re-initializing.
   A hook in an `if` *condition* or a `match` scrutinee is fine: it runs on every build.
2. **`set_during_build`** — `State::set` / `State::update` called synchronously in `build` requests
   a rebuild of the view that is currently building, i.e. an unconditional rebuild loop. The rule
   tracks which hook each binding came from (propagating through `.clone()`), because `use_ref`
   returns the same `State<T>` type with rebuilds disabled — a name-only check reports three false
   positives on a green tree. `.set()` inside a closure or an `async` block is deferred and not
   flagged.

Both rules are syntactic, so both have an escape hatch, and every diagnostic names the one that
silences it:

```rust
#[rusty::view(allow(conditional_hooks))]
#[rusty::view(allow(set_during_build))]
#[rusty::view(allow(conditional_hooks, set_during_build))]
```

An unknown rule name in `allow(..)` is an error, not a silently disabled lint. `#[rusty::view]`
never changes the code it annotates: it re-emits the impl block verbatim and appends diagnostics, so
a violation does not also produce "the trait `View` is not implemented" noise.

**Warnings are not available.** The only way a stable proc macro can emit a warning is a generated
`#[deprecated]` item, and the diagnostic then points at the attribute rather than the offending
line; `proc_macro::Diagnostic` is nightly-only and CI pins `dtolnay/rust-toolchain@stable`. So these
are hard errors plus the `allow(..)` hatch.

Applied to the five `impl View` blocks in `rusty/examples`. Library and test views are
un-annotated — plain `cargo build --workspace` compiles **zero** examples, so
`cargo clippy --workspace --all-targets` is the gate that actually reaches them.

Tests live in `rusty-macros`: `cargo test -p rusty-macros` runs 51 unit tests over `syn::parse_str`
fixtures plus a 19-case `trybuild` suite in `rusty-macros/tests/ui`. The `.stderr` files are
**exact-match snapshots** — to accept a changed message, delete the `.stderr` and re-run, then move
the file trybuild writes to **`rusty-macros/wip/`** back into `tests/ui/`. That path is the crate
root, not `tests/ui/wip/`, whatever the failure message says; `wip/` is gitignored. Never hand-edit
the snapshots. Seven
`t.pass(..)` cases carry the shapes that already exist in the repo (hook in an `if` condition,
`.set()` in a closure, `.set()` in `tokio::spawn(async move { .. })`, `.update()` on a `use_ref`
binding) and each `allow(..)` hatch, so a rule that starts over-reporting fails there.

`#[derive(Widget)]` also collects its diagnostics now, so multiple problems report at once instead
of one per compile. Five checks, two of which close holes that previously compiled clean with zero
warnings: a `Vec<Element>` / `Option<Vec<Element>>` field **not named `children`** (it gets no
`children_mut`, so `Element::assign_ids` never descends into it and its whole subtree loses IDs and
event registrations), and a struct with `#[event]` fields but no `#[prop]` fields and no `id` (it
serializes to just its type and `has<Event>` flags). The other three replace type errors from deep
inside generated code: `#[prop]` and `#[event]` on one field, an `#[event]` field that is not
`Option<..>`, and an `id` that is not `Option<String>`.

**`clippy.toml`** covers the one invariant the attribute cannot see: raw slot machinery
(`BuildContext::next_hook_index`, `BuildContext::reset`, `HookStore::get_or_init_state`) via
`disallowed_methods`, which is all stable clippy enforces — a real custom lint pass needs
`rustc_private` on nightly or `cargo-dylint`. Note the key is hyphenated (`disallowed-methods`)
while the lint is `disallowed_methods`, and it resolves **inherent methods as well as free
functions**. `clippy.toml` has no path scoping, so the legitimate definition sites in
`rusty/src/hooks/**`, `Runtime::build_view` and `BuildContext::child_view` carry
`#[expect(clippy::disallowed_methods, ..)]` rather than `#[allow(..)]`: an expectation that stops
being needed reports `unfulfilled_lint_expectations`, so the annotations cannot silently rot. Prefer
covering an invariant in `#[rusty::view]` over `clippy.toml` wherever both could.

## Frontend (src/frontend)
Vite+ toolchain, pnpm@10.33.0. Always `pnpm run <script>` or `pnpm exec vp` —
a globally installed `vp` may be an older version and `vp migrate` would
downgrade the project config.

CI's `frontend` job runs every command above except `pnpm format:check`, which is local-only. The `check:toolchain` lockstep check also runs in CI.

grouped in `renovate.json` as the "vite-plus toolchain" so they bump together:
`vite-plus` pins `vitest` exactly, so a partial bump desynchronizes the
toolchain. Five entries carry them — `devDependencies.vite`,
`devDependencies.vite-plus`, `devDependencies.vitest`, `pnpm.overrides.vite` and
`pnpm.overrides.vitest` — and all five must move to the same versions in one
change. `pnpm run check:toolchain` enforces this in CI: it compares all five
entries and checks `vitest` against the pin `vite-plus` declares on the registry.
It does not check freshness, which remains Renovate's job once its App is
installed. For local workstation drift detection, `pnpm run doctor` compares
installed versions in `node_modules` against declared versions in `package.json`
(CI cannot catch this since it installs onto a clean runner). As of 2026-08-01 Renovate is not installed on any Ivy-Interactive
repo, so `renovate.json` is a declaration of intent. CI's `renovate-liveness`
job fails once the config is 14 days old with no Renovate issue or PR: either
install https://github.com/apps/renovate or delete `renovate.json`. Do not
leave it as decoration — `Ivy-Web/.github/renovate.json` has sat inert since a
2024-03 `create-turbo` scaffold and has never opened a single PR. To check what `renovate.json` would actually do, see "Probing renovate.json" under `## CI` - the dry-run needs `GITHUB_COM_TOKEN` or it silently reports no GitHub Actions updates.


To verify which version is actually linked in `node_modules` (not a globally
installed `vp`):

```sh
cd src/frontend && pnpm exec vp --version
```
Beyond the vite-plus trio, `renovate.json` groups the remaining 117 npm entries
(86 dependencies + 21 devDependencies + 10 pnpm.overrides in `src/frontend/package.json`,
plus 1 devDependency in `e2e/package.json`) into a single weekly PR on Mondays.
Minor and patch updates ship in that group; majors and the `framer-motion` → `motion`
package rename appear only on the Dependency Dashboard. `@glideapps/glide-data-grid`
is pinned because its prerelease tags (alpha24, alpha9, alpha3…) sort backwards as strings,
so Renovate would offer `6.0.4-alpha24 → 6.0.4-alpha9` as a patch — a 501-day downgrade.
**The npm group rule must stay before the vite-plus rule** in `packageRules`: the array is
last-match-wins, so placing the npm group after the trio absorbs all five vite-plus entries
into the npm group and breaks their lockstep. Grouping is not optional — without it Renovate
opens 36 PRs, and it splits `pnpm.overrides` mirrors (e.g. `mermaid` and
`remark-mermaid-plugin>mermaid`) into separate branches even when they're the same version.

`pnpm run check:toolchain` compares the five manifest entries to each other; it reads no
`node_modules` and passes on a stale install. `pnpm run check:installed` compares the four exact
toolchain pins against what is actually installed, and runs automatically before `build`, `lint` and
`test`. If it fails, run `pnpm install` — a populated `node_modules` is not evidence of a current
one.

Git hooks are husky (`.husky/pre-commit` + `package.json`'s `lint-staged`). Vite+'s `vp staged` / `staged` config is intentionally unused — do not run `vp config`, which would install a competing `.vite-hooks` tree.

## E2E (e2e)

`e2e/` is a separate **npm** project (it has `package-lock.json` and no `packageManager` — do not
reach for pnpm here) driving the `target/debug/widget_harness` binary through Playwright. It is the
Rust widgets' only real client: a new widget is invisible to it until it has a `case` arm in
`e2e/app/index.html` **and** an `*App` + `match` arm in `rusty-server/src/bin/widget_harness.rs`.

CI runs it as the `e2e` job, whose first step is `node scripts/check-harness-script.js` — it
extracts the file's single inline `<script>` and `node --check`s it, before building or installing
anything. **Run that script yourself after resolving any conflict in `e2e/app/index.html`.** The file
has been left unparseable twice by hand-resolved merge conflicts (`7b7e981`, `0939697`), each time
silently zeroing the entire suite: `document.querySelectorAll('[data-widget-type]')` returns nothing
and every spec dies in `waitForSelector`, pointing at no cause. `npx playwright test --list` exits 0
on a broken file, because the Playwright loader never looks at the HTML.

Three gates cover this file, and each sees something the others miss. `cargo test`'s
`harness_client_is_loadable` checks structure — script tag count, brace balance, duplicate case
labels — without a browser or a Playwright install. `node --check` catches what balances but does not
parse: the stray `break;` at `0939697` left braces even and labels unique, so structure alone reports
it clean. And only running the suite catches logic that parses *and* balances but is wrong —
`206886a` reverted plan 00080's `avatar` `data-size` fix two minutes after it landed, and no static
check can see that. Do not treat any one of the three as covering for the others.

Locally: `cargo build -p rusty-server --bin widget_harness`, then from `e2e/`, `npm ci`,
`npx playwright install chromium`, `npx playwright test`. A stale harness binary reports
`Unknown widget: <name>`, which is easy to mistake for a code fault.

## CI

`.github/workflows/ci.yml` runs build, test, clippy, `cargo fmt --all -- --check`,
`.github/workflows/ci.yml` runs build, test, clippy, `cargo fmt --all -- --check`,
a test-inventory check (see below), frontend checks, the `e2e` Playwright suite, and renovate-liveness on every
push to `main` and every PR. All checks report independently — a failure in one does not skip the rest.
`build`, `frontend`, `e2e` and `renovate-liveness` are separate jobs, not steps of one. When any of those jobs fails on a push to `main`, `alert-on-red-main` opens or comments on a `ci-red` issue - it depends on all of them, so no job's failure is silent. A weekly `cargo-majors` job (`schedule`, plus `workflow_dispatch`) reports Cargo dependencies whose latest stable release is outside the major series declared in the manifests. It is report-only. This exists because `renovate.json` parks all cargo updates, and Renovate omits parked dependencies from the Dependency Dashboard entirely - a parked major is invisible, not a checkbox. As of 2026-08-02: `syn` `^2` -> 3.0.3, `tower-http` `^0.6` -> 0.7.0, `tokio-tungstenite` `^0.29` -> 0.30.0.

`main` has no branch protection and no rulesets:

```bash
gh api repos/Ivy-Interactive/Rusty-Framework/branches/main/protection  # 404
gh api repos/Ivy-Interactive/Rusty-Framework/rulesets                  # []
```

**20 of 20 PRs (#27-#46) on 2026-08-01 merged 13-74s before their last check finished.**
Agents merge with `gh pr merge --merge --admin` immediately after opening the PR, so the
check result arrives after the merge and nothing reads it. A required status check alone
will not stop this: `--admin` bypasses requirements for admins. The fix needs
**`enforce_admins: true`** to block merges while checks are pending. Run
`.github/protect-main.sh` (requires repo ADMIN) to configure protection on `main`.

`cargo fmt --all -- --check` needs a prior `cargo build`: `rusty-docs/src/generated/`
is gitignored and emitted by `rusty-docs/build.rs`, so rustfmt fails to resolve
`mod generated` on a clean checkout.

### The Test inventory gate

`scripts/check-test-inventory.sh <base-ref>` fails when a test that exists at `<base-ref>`
is gone from the working tree. It runs as the `Test inventory` step of the `build` job,
against `github.event.pull_request.base.sha` on a PR and `github.event.before` on a push
to `main`.

None of the other four cargo gates can see a deleted test — a test module removed wholesale
takes its own assertions with it. Measured by deleting the 8-test `#[cfg(test)]` module from
`rusty/src/server/ws.rs`: `cargo build`, `cargo test` (384 passed, down from 392),
`cargo clippy -- -D warnings` and `cargo fmt -- --check` all exit **0**. The new gate exits 1
and names all 8. **It compares names, not counts**, because the merge that motivated it
(`53dff77`) deleted 36 tests while raising the workspace total from 222 to 266 — a
"tests must not decrease" check passes it.

Deleting a test is legitimate. The gate reports names for a human to judge rather than
blocking renames, and since `main` has no branch protection (see above) it informs rather
than blocks: **rename, move or drop a test on purpose and say so in the commit message.**
What it exists to catch is the test module that vanishes while someone hand-resolves a merge
conflict and commits without re-running the gates.

Two things it needs, both of which fail silently if you drop them:

- **`fetch-depth: 0` on the `build` job's checkout.** The script extracts the base with
  `git archive`, which fails with `not a valid object name` on the default shallow clone.
- **Its own `CARGO_TARGET_DIR` for the base tree**, which the script sets. `rusty-docs/build.rs`
  only reruns when the target dir holds no cached fingerprint, so sharing a warm one makes the
  base tree fail `E0583: file not found for module 'generated'`.

`scripts/test-check-test-inventory.sh` is its harness (the `Test-inventory gate harness` step):
6 black-box cases over a throwaway cargo workspace, including a base tree that does not compile
— the case where an earlier prototype printed `0 tests at <base>` and exited 0 on a tree it had
never inspected.

`cargo test --workspace` now asserts `e2e/app/index.html` is structurally loadable (matching script tag count, brace balance, no duplicate case labels), which catches breakages a `pageerror`-only check misses — a duplicated `</script>` throws no pageerror yet renders half the code as page text.

### Probing renovate.json

`renovate.json` has no test and no CI check on its *contents* (`renovate-liveness` only checks the
file exists and the App has run). To see what it would actually do, dry-run Renovate against a
scratch copy - never the real worktree:

```bash
rm -rf ~/rf-scratch/reno && mkdir -p ~/rf-scratch/reno
git archive HEAD | tar -x -C ~/rf-scratch/reno      # MSYS_NO_PATHCONV=1 on Git Bash
cd ~/rf-scratch/reno
# drop husky's `prepare` script or renovate's install step fails
node -e "const fs=require('fs');for(const f of ['src/frontend/package.json','e2e/package.json']){const j=JSON.parse(fs.readFileSync(f,'utf8'));if(j.scripts?.prepare){delete j.scripts.prepare;fs.writeFileSync(f,JSON.stringify(j,null,2))}}"
git init -q . && git add -A && git -c user.email=a@b -c user.name=a commit -qm probe
GITHUB_COM_TOKEN=$(gh auth token) LOG_LEVEL=debug \
  npx --yes renovate@latest --platform=local --dry-run=full > log.txt 2>&1
grep -o '"branchName": "renovate/[^"]*"' log.txt | sort -u    # the PRs it would open
npx --yes --package=renovate renovate-config-validator renovate.json
```

**`GITHUB_COM_TOKEN` is mandatory, and omitting it fails silently.** Without it every
GitHub-sourced dependency is dropped with `skipReason: "github-token-required"` and `updates: []`
- the `github-tags` and `github-digest` datasources are unauthenticated-rate-limited. Measured at
`9c56744` with Renovate 44.5.3: six deps skipped (`actions/checkout`, `actions/github-script`,
`actions/setup-node`, `pnpm/action-setup`, `Swatinem/rust-cache`, and `node` from
`setup-node`'s `node-version: 24` input). With the `github-actions` manager enabled, a tokenless
run reports **zero** updates while a tokened run finds two majors, `actions/checkout` v4 -> v7 and
`actions/github-script` v7 -> v9. Never conclude "no action updates" from an untokened run.

The skip is easy to misread because `packageRules`' `{"matchPackageNames": ["*"], "enabled": false}`
produces the same empty `updates: []` via `skipReason: "disabled"`. Both yield zero branches, so
read the `skipReason`, not the branch list, to tell "manager is off" from "token is missing".
`dtolnay/rust-toolchain@stable` is a third shape: `github-digest` emits a `warnings` entry
(`Failed to look up github-digest package ... no-result`), not a `skipReason`.

Two log lines are expected and are not failures: "The platform you're using (local) does
not support local presets", and a "Would commit files to onboarding branch" line.
