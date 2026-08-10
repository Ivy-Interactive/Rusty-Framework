# src/frontend/e2e

This suite is not run by any CI job — the `frontend` job in `.github/workflows/ci.yml` runs lint,
typecheck, build, bundle-budget, test, and format-check, but never `pnpm e2e`. 8 of its 9 specs
are unconditionally skipped (`test.skip(true, ...)`), one citing "while PR #??? stabilizes" — an
inherited placeholder with no such PR.

The suite targets a `vp dev` server against `IVY_HOST` (`https://localhost:5010` by default), the
inherited .NET backend from Ivy-Framework. That backend does not exist in this repo, so this
suite does not — and cannot — exercise the Rust framework.

This should be either wired into the `frontend` CI job or deleted; see
[`docs/repository-structure.md`](../../docs/repository-structure.md) for the full context. The
skipped specs are left as-is here: unskipping them would fail against a backend that isn't
present, and turning a silent skip into a red gate is a separate decision from documenting the
current state.
