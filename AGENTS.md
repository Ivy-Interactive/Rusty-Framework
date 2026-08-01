# Rusty-Framework Agent Instructions

## Verify

Rusty-Framework uses a hybrid Rust/Node frontend stack. Always verify both sides:

cargo test --workspace

cargo fmt --all

## Lint
cargo clippy --workspace --all-targets -- -D warnings

## Frontend (src/frontend)
Vite+ toolchain, pnpm@10.33.0. Always `pnpm run <script>` or `pnpm exec vp` —
a globally installed `vp` may be an older version and `vp migrate` would
downgrade the project config.

`vite-plus`, `@voidzero-dev/vite-plus-core` (aliased as `vite`) and `vitest` are
grouped in `renovate.json` as the "vite-plus toolchain" so they bump together:
`vite-plus` pins `vitest` exactly, so a partial bump desynchronizes the
toolchain. Five entries carry them — `devDependencies.vite`,
`devDependencies.vite-plus`, `devDependencies.vitest`, `pnpm.overrides.vite` and
`pnpm.overrides.vitest` — and all five must move to the same versions in one
change. Renovate does this automatically once its GitHub App is installed on the
repo; until then, keep them in lockstep by hand.

Git hooks are husky (`.husky/pre-commit` + `package.json`'s `lint-staged`). Vite+'s `vp staged` / `staged` config is intentionally unused — do not run `vp config`, which would install a competing `.vite-hooks` tree.
