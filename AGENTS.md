# AGENTS.md

## Build
cargo build --workspace

## Test
cargo test --workspace

## Format
cargo fmt --all

## Lint
cargo clippy --workspace -- -D warnings

## Frontend (src/frontend)
Vite+ toolchain, pnpm@10.33.0. Always `pnpm run <script>` or `pnpm exec vp` —
a globally installed `vp` may be an older version and `vp migrate` would
downgrade the project config.
