# Frontend

**Node.js Version Requirement**: This project requires Node.js version 24 or greater (matching
`.github/workflows/ci.yml`), and uses **pnpm** with the **Vite+** (`vp`) toolchain.

## Development

Install dependencies from `src/frontend`:

```bash
pnpm install --frozen-lockfile
```

Always run the toolchain via `pnpm run <script>` or `pnpm exec vp` — never a globally installed
`vp`. A global `vp` may be an older version, and `vp migrate` would downgrade the project config.

```bash
pnpm run dev             # start the dev server
pnpm run build           # typecheck (tsc -b) then bundle for production
pnpm lint                # lint
pnpm test                # unit tests
pnpm format:check        # formatting check
```

## Developer Logging

The frontend includes a comprehensive logging system for debugging and development purposes. Detailed logging can be controlled via browser console commands.

### Console Commands

Open the browser console (F12 → Console tab) and use these commands:

```javascript
// Check current developer options
getDeveloperOptions();
// Returns: { showDetailedLogging: false }

// Toggle detailed logging on/off
toggleDeveloperLogging();
// Returns: true (if enabled) or false (if disabled)
// Also logs: "Developer logging enabled" or "Developer logging disabled"
```

### What Gets Logged

When detailed logging is enabled, you'll see debug messages for:

- **Select Input Interactions**: Value changes, conversions, clear operations
- **SignalR Communication**: Message processing, updates, events
- **Widget Tree Operations**: XML conversion, patches, updates
- **Authentication**: access token operations, theme changes
- **Error Handling**: Connection issues, parsing errors

### Log Levels

- **Debug**: Detailed information (controlled by `showDetailedLogging`)
- **Info**: General information (always visible)
- **Warn**: Warning messages (always visible)
- **Error**: Error messages (always visible)

### Persistence

Developer options are stored in localStorage and persist across:

- Page refreshes
- Browser sessions
- Browser restarts

## Code Quality

The frontend project uses **Vite+** integrated tools (**Oxlint** and **Oxfmt**) for high-performance code quality and formatting, alongside automatic pre-commit hooks.

### Pre-commit Hooks

We use a Husky npm package to set up the repo's pre-commit hook. It lints and formats staged frontend files and runs rustfmt on staged .rs files.
The frontend step requires `node` on `PATH` (the local `vp` shim execs it); if it is missing the hook stops with an explicit error. Staged-Rust-only and conflict-marker commits do not need node.

The active hook is `.husky/pre-commit` (`core.hooksPath` points at `.husky/_`), and its frontend step reads per-glob commands from the `lint-staged` key in `package.json`. Edit those two files. Vite+'s own `vp staged` runner is deliberately **not** used: husky's installer rewrites `core.hooksPath` on every `pnpm install`, so the two cannot coexist, and `.husky/pre-commit` also carries a merge-conflict-marker check that a `staged` glob map cannot express.

For Rust files, the hook runs `rustfmt --edition 2021 --config skip_children=true` on staged `.rs` files. Fully staged files are auto-formatted and re-added; partially staged files (where the worktree has unstaged edits) are checked as they exist in the index, never rewritten, and will block the commit with "Run: cargo fmt --all" if unformatted. If `rustfmt` is not on `PATH`, the Rust block is skipped.

Hooks are installed automatically by the `prepare` script when you run `pnpm install` in `src/frontend`. Ideally, you would not then need to run any formatting or lint commands as it will be done for you. In case you want to manually run them, you still can.

**`core.hooksPath` is not configurable here.** The `prepare` script runs husky's installer, which sets `core.hooksPath` to `src/frontend/.husky/_` unconditionally — it never checks the current value. If you point `core.hooksPath` somewhere else, the next `pnpm install` in `src/frontend` silently puts it back, with no warning. This is deliberate: it keeps `.husky/pre-commit` the single authoritative hook. To disable hooks for one command use `git commit --no-verify`, or set `HUSKY=0` to skip the install step (`index.js` returns early on `HUSKY=0`).

### Code Formatting

Format all files with Oxfmt using the Vite+ CLI:

```bash
vp fmt .
```

Check if files are properly formatted:

```bash
vp fmt --check .
```

### Linting

Check for linting issues with Oxlint using the Vite+ CLI:

```bash
vp lint .
```

Automatically fix linting issues:

```bash
vp lint --fix .
```

### Configuration Files

- `vite.config.mjs` - Contains Vite+ syntax formatting and linting preferences
- `package.json` - Contains execution scripts

### Checking the Installed Toolchain Version

To see which version of a package is actually in use, ask pnpm — do **not** list the store:

```sh
pnpm ls vite-plus     # -> vite-plus@0.2.7
```

`ls node_modules/.pnpm | grep vite-plus` is **not** a reliable diagnostic. `.pnpm` keeps every
generation it has ever linked, so a freshly repaired tree still lists the stale one; only the
symlink at `node_modules/vite-plus` reflects what actually resolves.

Superseded generations are inert — nothing links them — but they make that listing misleading. To
remove them:

```sh
pnpm prune --ignore-scripts
```

`--ignore-scripts` matters here: `pnpm prune` re-runs the `prepare` script, and this project's
`prepare` is the husky installer, which rewrites `git config core.hooksPath` unconditionally.
`pnpm store prune` is a **different** command — it cleans the global content-addressable store and
leaves `node_modules/.pnpm` untouched.

## Module Graph and Lazy Loading

Most widgets in `src/widgets/widgetMap.ts` are code-split with `lazyWithRetry(() => import("..."))`.
Whether that actually splits anything is decided by the module graph, not by the `import()` call, and
it is easy to defeat by accident. This section is the rule, the two ways it breaks, and the check.

### The rule

A widget loaded with `lazyWithRetry(() => import("..."))` gets its own lazily-loaded chunk **only if
no eagerly-reachable module statically imports a runtime value from it**. One such import anywhere in
the tree is enough: the static edge wins, the module is pulled into the eager graph, and the dynamic
`import()` becomes a no-op that resolves something already loaded.

`widgetMap.ts` is where this goes wrong, because it is eagerly loaded and holds both halves: roughly
65 widgets imported statically at the top of the file, and 49 `lazyWithRetry(() => import(...))` call
sites in the map below. An eager import that reaches a lazy widget's module is all it takes.

### Two ways the edge gets created

**1. Through a sibling barrel.** `widgetMap.ts` imports an eager widget from a barrel, and that
barrel also re-exports the lazy widget. Importing _anything_ from the barrel drags in everything it
re-exports. This is what happened with `@/widgets/lists`: it re-exported both `ListItemWidget` (eager)
and `ListWidget` (lazy), so importing the former pulled in the latter. There are two fixes, and both
are in place for lists - import the concrete module at the call site, and keep the lazy widget out of
the barrel:

```ts
import { ListItemWidget } from "@/widgets/lists/ListItemWidget"; // not "@/widgets/lists"
```

**2. From the same file.** The eager exports live in the _same file_ as the lazy widget. Bypassing the
barrel cannot help here - any import of the eager widget resolves to the module that also holds the
lazy one. Fix by splitting the eager exports into their own files, as `chat/` does with
`ChatMessageWidget.tsx`, `ChatLoadingWidget.tsx` and `ChatStatusWidget.tsx`. Once split, keep the
lazy widget out of the barrel: `chat/index.ts` deliberately does not re-export `ChatWidget`.

Type-only references are free, in either direction. A `type` does not exist at runtime, so it creates
no edge, which is why `ChatWidget.tsx` may safely import `ChatMessageWidgetProps` from the file it was
split out of. Write it as `import type { ... }`: a plain `import` of a type-only binding is erased too
and does not change the bundle, but the `type` keyword states the intent and is required if
`verbatimModuleSyntax` is ever enabled - under that flag the plain form is a hard `TS1484` error.

Note the direction that matters. It is the **eager module importing a value from the lazy one** that
defeats the split. The reverse is fine: a lazy widget may import real values from eager modules, and
in fact does - `ChatWidget.tsx` imports `Button` and `ChatInput` that way.

### How to check

**The build will fail.** `vite.config.mjs` has an `assert-lazy-chunks` plugin that reads the module
graph in `generateBundle` and fails the build if any first-party dynamically imported module lands in
a chunk that is also statically imported, or if it is merged into the entry chunk (exit 1, with the
source file and chunk name reported). This replaces an earlier plugin that promoted Rolldown's
`INEFFECTIVE_DYNAMIC_IMPORT` warning: that warning is never emitted on `vite-plus` 0.2.7, so the old
gate was silent while both known bug shapes built with exit 0. Reading the module graph works on the
pinned version.

Because it reads the emitted graph, it is blind to an edge that never reaches the graph: a named
import of a lazy module whose binding is never used is elided by Rolldown, so no eager edge exists
and the build correctly exits 0. `pnpm lint` and `tsc -b` both reject that code anyway, and the
source-level test below reports it directly.

Chunk size is not a signal. The defeated chunk is still emitted at close to its normal size (13,952
bytes vs 13,925 correct), so the "69-byte facade" symptom described in older notes no longer appears.

To manually confirm a specific widget is lazy after `pnpm run build`:

```bash
cd src/frontend
for w in ChatWidget ListWidget; do
  c=$(ls dist/assets | grep -E "^${w}-[^-]*\.js$" | head -1)
  grep -qF -e "import\"./$c" -e "from\"./$c" dist/assets/*.js &&
    echo "EAGER (lazy loading defeated): $w [$c]"
done
```

Silence means the widget is genuinely lazy. The distinction is that a lazy edge is emitted as
``import(`./ChatWidget-<hash>.js`)`` with backticks, whereas an eager one appears as a
double-quoted `import"./ChatWidget-<hash>.js"` or `from"./ChatWidget-<hash>.js"`. Widen the `for`
list to check other widgets; names that share a chunk with another widget have no chunk of their own
and will simply not match.

A cheaper guard covering every lazy entry is `src/widgets/__tests__/lazyWidgetBarrels.test.ts`. It
walks static, non-`type` imports from `src/index.tsx` and fails if any `widgetMap.ts` `import()`
target is eagerly reachable, printing the whole import chain that created the edge. It reads source
only, so it needs no build and runs in milliseconds - a fast complement to the build-time gate
above, not a replacement for it.

### Barrels with no importers are inert

`widgetMap.ts` is the only consumer of a top-level widget barrel in the tree, apart from
`@/widgets/rowAction` (used by `tree/TreeItem.tsx` and `dataTables/`). Six of the 27 top-level barrels
have no importer at all, so they cannot defeat anything no matter what they re-export. There is no
need to pre-emptively narrow them.

A barrel whose only runtime export is the lazy widget itself (plus `export type` declarations) is
harmless while nothing imports it, but rewriting the `import()` to name the concrete module alone
does not protect it - the barrel's own `export { Widget }` re-export becomes the static edge once
an eager importer appears. Both halves must be applied: narrow the barrel and point the dynamic
import at the concrete module. Four barrels (`calendar/`, `kanban/`, `tree/`, `layouts/sidebar/`)
currently re-export a lazy widget but have zero production importers and remain intentionally
untouched.

## Testing

This project uses Vitest (via Vite+) for unit testing and Playwright for end-to-end testing.

### Unit Testing with Vitest

Run unit tests interactively using Vite+:

```bash
vp test
```

Unit tests are configured to run only on files ending with `.test.ts`. Place your unit test files alongside your source code with the `.test.ts` extension.

### End-to-End Testing with Playwright

### Prerequisites

Make sure you're in the frontend directory:

```bash
cd frontend
```

### Install Dependencies

```bash
vp install
```

### Install Playwright Browsers

```bash
vp exec playwright install --with-deps
```

### Running Tests

Run all e2e tests:

```bash
vp run e2e
```

Run only Ivy.Docs e2e tests:

```bash
vp run e2e:docs
```

Run only Ivy.Samples e2e tests:

```bash
vp run e2e:samples
```

Run tests in a specific browser:

```bash
vp run e2e -- --project=chromium
vp run e2e -- --project=firefox
vp run e2e -- --project=webkit
```

Run tests in headed mode (to see the browser):

```bash
vp run e2e -- --headed
```

Run tests in debug mode:

```bash
vp run e2e -- --debug
```

Run a specific test file:

```bash
vp run e2e -- example.spec.ts
```

### Test Reports

View the HTML test report:

```bash
vp run e2e -- --reporter=html
# Then open the report
vp exec playwright show-report
```

### Test Files

- `e2e/` - End-to-end test files

### CI/CD

Tests are automatically run in GitHub Actions on push to main/master branches and pull requests. The CI pipeline includes:

1. Code formatting checks (`vp fmt --check .`)
2. Linting checks (`vp lint .`)
3. Unit tests (`vp test`)
4. Playwright end-to-end tests

## Available Commands and Scripts

| Command/Script       | Description                           |
| -------------------- | ------------------------------------- |
| `vp dev`             | Start development server              |
| `vp run build`       | Build for production (typecheck + vp) |
| `vp preview`         | Preview production build              |
| `vp test`            | Run unit tests with Vitest            |
| `vp run e2e`         | Run all end-to-end tests              |
| `vp run e2e:docs`    | Run Ivy.Docs end-to-end tests         |
| `vp run e2e:samples` | Run Ivy.Samples end-to-end tests      |
| `vp lint .`          | Check for linting issues              |
| `vp lint --fix .`    | Fix linting issues automatically      |
| `vp fmt .`           | Format all files with Oxfmt           |
| `vp fmt --check .`   | Check if files are properly formatted |
