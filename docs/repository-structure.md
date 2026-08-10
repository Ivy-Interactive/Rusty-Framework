# Repository Structure

This document describes the top-level layout of the repo, why it looks the way it does, and the
structural debt that follows from that history. It is organised by *tree*; for gate-by-gate
operational detail (what CI runs, how to run it locally, known failure modes), see
[AGENTS.md](../AGENTS.md).

## Tree

| Path | What it is | Toolchain | CI job |
|------|------------|-----------|--------|
| `rusty/` | Core framework: `core`, `hooks`, `server`, `shared`, `views`, `widgets` (29 widget files) | cargo | `build` |
| `rusty-macros/` | `#[derive(Widget)]`, `#[rusty::view]` proc macros (+ 19-case `trybuild` suite) | cargo | `build` |
| `rusty-ivyml/` | `ivyml!` / `ivyml_file!` declarative markup macros | cargo | `build` |
| `rusty-filter/` | Filter query lexer/parser/AST and evaluator | cargo | `build` |
| `rusty-server/` | Server binary, including `src/bin/widget_harness.rs` (the e2e test harness) | cargo | `build`, `e2e` |
| `rusty-docs/` | Docs site binary; `build.rs` codegen writes `src/generated/` (gitignored) | cargo | `build` |
| `rusty-desktop/` | wry/tao native shell behind the optional `shell` feature; `assets/index.html` | cargo | `build`, `desktop-shell` |
| `src/frontend/` | React/TS app: 532 TS files, 107 npm deps, own `.husky`/`.gitignore`/`.gitattributes` | pnpm | `frontend` |
| `e2e/` | Separate npm Playwright project; `app/index.html` (955-line renderer) drives `widget_harness` | npm | `e2e` |
| `scripts/` | Gate/harness scripts (sh + node) invoked by CI and locally | sh / node | — |
| `.github/` | `ci.yml` (build, frontend, e2e, desktop-shell, renovate-liveness, cargo-majors, alert-on-red-main) + `protect-main.sh` | — | — |

## Why crates are at the root and `src/` is not

The initial scaffold (`8f4752b`, "Rust port of Ivy-Framework") carried over Ivy-Framework's .NET
layout, which puts everything under `src/` (`src/Ivy`, `src/Ivy.Docs`, `src/frontend`). The Rust
port placed its crates at the repo root instead — the idiomatic location for a Cargo workspace —
but left `src/frontend` where it was. `src/` today holds exactly one child and means "the
non-Rust part of the repo," a name that no longer describes anything since it isn't a sibling to
any other `src/*` directory.

## The two `e2e` trees

| | `e2e/` | `src/frontend/e2e/` |
|---|---|---|
| Package manager | npm (`package-lock.json`) | pnpm (part of `src/frontend`) |
| Config | `e2e/playwright.config.ts` | `src/frontend/playwright.config.ts` |
| `testDir` | `./tests` | `./e2e` |
| Spec count | 20 | 9 |
| Drives | `target/debug/widget_harness` | `vp dev` server against `IVY_HOST` |
| Run by CI | Yes — `e2e` job | No |

`src/frontend/e2e` is dead: the `frontend` CI job runs lint, typecheck, build, bundle-budget,
test, and format-check, but never `pnpm e2e`. 8 of its 9 specs are `test.skip(true, ...)`, one
citing "while PR #??? stabilizes" — an inherited placeholder. See
[`src/frontend/e2e/README.md`](../src/frontend/e2e/README.md) for the disposition of this suite.

For everything else about `e2e/` — the harness binary, the merge-conflict failure mode, why
`npx playwright test --list` can pass on a broken renderer — see AGENTS.md's
[`## E2E`](../AGENTS.md) section rather than restating it here.

## Ownership map for adding a widget

A new widget touches four places, none of which reference the others:

1. `rusty/src/widgets/*.rs` — the widget's Rust implementation and `#[derive(Widget)]`.
2. `rusty-server/src/bin/widget_harness.rs` — an `*App` and `match` arm so the e2e harness can
   render it.
3. `e2e/app/index.html` — a `case` arm in the renderer so the harness's Playwright suite sees it.
4. `rusty-desktop/assets/index.html` — the same `case` arm, if the widget needs the desktop shell
   (`rusty-desktop/src/assets.rs` documents this file's copy relationship to `e2e/app/index.html`).

This is the single most-duplicated act in the repo. See the "Deduplicate the 955-line renderer"
recommendation on the plan that produced this document for a proposed fix.

## Known structural debt

1. **`src/` holds exactly one child** — see "Why crates are at the root" above.
2. **Two directories named `e2e`, two Playwright configs, two package managers** — see "The two
   `e2e` trees" above.
3. **`src/frontend/e2e` is dead** — not run by any CI job; 8 of 9 specs unconditionally skipped.
4. **The 955-line renderer exists twice** — `e2e/app/index.html` and
   `rusty-desktop/assets/index.html` differ by 35 diff lines and are maintained by hand.
5. **`src/frontend/README.md` documented a .NET repo** — fixed by this same change; see its git
   history for the previous (inherited, incorrect) content.
6. **Two stale root files** — `.gitattributes` and `.gitignore` referenced paths that don't
   exist or omitted crates that do; fixed by this same change.

## The undecided question: `src/frontend`'s backend

`src/frontend` connects via `@microsoft/signalr` to `IVY_HOST || https://localhost:5010`, and its
dev server fetches HTML from that host on startup (`injectMeta` in
[`vite.config.mjs`](../src/frontend/vite.config.mjs)). No Rust crate in this repo serves its
`dist/`. The client the Rust servers actually use is the vanilla-JS `e2e/app/index.html`. So 532
TS files and 5 of the 9 required verifications currently gate a React app whose backend is a .NET
host that does not exist in this repo. Whether that is a staging area for a port-in-progress or
dead weight is a product decision, not a refactor — this document records the finding and leaves
the call to a human.
