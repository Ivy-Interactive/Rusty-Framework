# Rusty-Framework Agent Instructions

## Verify

Rusty-Framework uses a hybrid Rust/Node frontend stack. Always verify both sides:

cargo test --workspace

cargo fmt --all

**Note:** Rust formatting is enforced on pre-commit via `.husky/pre-commit`. Staged `.rs` files are auto-formatted with `rustfmt --edition 2021 --config skip_children=true` and re-added. Partially staged files are checked only and will block the commit if unformatted.

## Lint
cargo clippy --workspace --all-targets -- -D warnings

## Frontend (src/frontend)
Vite+ toolchain, pnpm@10.33.0. Always `pnpm run <script>` or `pnpm exec vp` —
a globally installed `vp` may be an older version and `vp migrate` would
downgrade the project config.

CI runs these three, from `src/frontend`, after `pnpm install --frozen-lockfile`:

pnpm lint --max-warnings=0
pnpm exec tsc -b
pnpm test

`vite-plus`, `@voidzero-dev/vite-plus-core` (aliased as `vite`) and `vitest` are
grouped in `renovate.json` as the "vite-plus toolchain" so they bump together:
`vite-plus` pins `vitest` exactly, so a partial bump desynchronizes the
toolchain. Five entries carry them — `devDependencies.vite`,
`devDependencies.vite-plus`, `devDependencies.vitest`, `pnpm.overrides.vite` and
`pnpm.overrides.vitest` — and all five must move to the same versions in one
change. Renovate does this automatically once its GitHub App is installed on the
repo; until then, keep them in lockstep by hand.

## CI

`.github/workflows/ci.yml` runs build, test, clippy and `cargo fmt --all -- --check`
on every push to `main` and every PR. All four report independently — a failure in one
does not skip the rest.

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
