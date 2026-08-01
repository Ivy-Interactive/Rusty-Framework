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
the file trybuild writes to `tests/ui/wip/` back into `tests/ui/`. Never hand-edit them. Seven
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

CI's `frontend` job runs the first four of the frontend commands above; `pnpm format:check`
is local-only.

`vite-plus`, `@voidzero-dev/vite-plus-core` (aliased as `vite`) and `vitest` are
grouped in `renovate.json` as the "vite-plus toolchain" so they bump together:
`vite-plus` pins `vitest` exactly, so a partial bump desynchronizes the
toolchain. Five entries carry them — `devDependencies.vite`,
`devDependencies.vite-plus`, `devDependencies.vitest`, `pnpm.overrides.vite` and
`pnpm.overrides.vitest` — and all five must move to the same versions in one
change. `pnpm run check:toolchain` enforces this in CI: it compares all five
entries and checks `vitest` against the pin `vite-plus` declares on the registry.
It does not check freshness, which remains Renovate's job once its App is
installed. As of 2026-08-01 Renovate is not installed on any Ivy-Interactive
repo, so `renovate.json` is a declaration of intent. CI's `renovate-liveness`
job fails once the config is 14 days old with no Renovate issue or PR: either
install https://github.com/apps/renovate or delete `renovate.json`. Do not
leave it as decoration — `Ivy-Web/.github/renovate.json` has sat inert since a
2024-03 `create-turbo` scaffold and has never opened a single PR. To check what `renovate.json` would actually do, see "Probing renovate.json" under `## CI` - the dry-run needs `GITHUB_COM_TOKEN` or it silently reports no GitHub Actions updates.

Git hooks are husky (`.husky/pre-commit` + `package.json`'s `lint-staged`). Vite+'s `vp staged` / `staged` config is intentionally unused — do not run `vp config`, which would install a competing `.vite-hooks` tree.

## CI

`.github/workflows/ci.yml` runs build, test, clippy, `cargo fmt --all -- --check`,
frontend checks, and renovate-liveness on every push to `main` and every PR. All checks
report independently — a failure in one does not skip the rest. A weekly `cargo-majors` job (`schedule`, plus `workflow_dispatch`) reports Cargo dependencies whose latest stable release is outside the major series declared in the manifests. It is report-only. This exists because `renovate.json` parks all cargo updates, and Renovate omits parked dependencies from the Dependency Dashboard entirely - a parked major is invisible, not a checkbox. As of 2026-08-02: `syn` `^2` -> 3.0.3, `tower-http` `^0.6` -> 0.7.0, `tokio-tungstenite` `^0.29` -> 0.30.0.

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
