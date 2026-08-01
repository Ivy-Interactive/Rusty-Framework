# Rusty-Framework Agent Instructions

## Verify

Rusty-Framework uses a hybrid Rust/Node frontend stack. Always verify both sides:

cargo test --workspace

cargo fmt --all

**Note:** Rust formatting is enforced on pre-commit via `.husky/pre-commit`. Staged `.rs` files are auto-formatted with `rustfmt --edition 2021 --config skip_children=true` and re-added. Partially staged files are checked as they exist in the index, never rewritten, and will block the commit with "Run: cargo fmt --all" if unformatted.

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

`main` has no branch protection and no rulesets, so a PR can be (and routinely is)
merged before its check run finishes. Between 19:15Z and 20:52Z on 2026-08-01,
11 of 11 PRs merged 10-74s ahead of their result and nine of them were red. Until a
required status check is configured, read the PR's check result before merging
(`gh pr checks <n> --watch`) rather than assuming a green PR page means a green build.

`cargo fmt --all -- --check` needs a prior `cargo build`: `rusty-docs/src/generated/`
is gitignored and emitted by `rusty-docs/build.rs`, so rustfmt fails to resolve
`mod generated` on a clean checkout.
