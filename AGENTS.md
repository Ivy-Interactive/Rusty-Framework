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
2024-03 `create-turbo` scaffold and has never opened a single PR. To check what `renovate.json` would actually do, see "Probing renovate.json" under `## CI` - the dry-run needs `GITHUB_COM_TOKEN` or it silently reports no GitHub Actions updates. To check what is actually installed, use `pnpm ls <pkg>`, not `ls node_modules/.pnpm` — `.pnpm` retains superseded generations, so that listing shows stale versions even on a correct tree. Clear them with `pnpm prune --ignore-scripts`, not `pnpm store prune`, which only touches the global store.

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
