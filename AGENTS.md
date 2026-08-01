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
2024-03 `create-turbo` scaffold and has never opened a single PR.

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

Git hooks are husky (`.husky/pre-commit` + `package.json`'s `lint-staged`). Vite+'s `vp staged` / `staged` config is intentionally unused — do not run `vp config`, which would install a competing `.vite-hooks` tree.

## CI

`.github/workflows/ci.yml` runs build, test, clippy, `cargo fmt --all -- --check`,
frontend checks, and renovate-liveness on every push to `main` and every PR. All checks
report independently — a failure in one does not skip the rest.

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
