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
