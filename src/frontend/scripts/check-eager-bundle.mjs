// Walks static import edges from the entry chunk named in dist/index.html and fails if the total
// eagerly-fetched JS exceeds MAX_EAGER_BYTES. A chunk is eager iff a static import edge reaches it
// from the entry: `manualChunks` cannot change that, only deleting the edge can. See README.md.
import { readFileSync, readdirSync, statSync } from "node:fs";
import { basename, join } from "node:path";
import { fileURLToPath } from "node:url";

const MAX_EAGER_BYTES = 2_713_600;

const dist = process.argv[2] ?? fileURLToPath(new URL("../dist", import.meta.url));
const assetsDir = join(dist, "assets");

let html;
try {
  html = readFileSync(join(dist, "index.html"), "utf8");
} catch {
  console.error(`No build found at ${dist}. Run \`pnpm run build\` first.`);
  process.exit(2);
}

const entries = [...html.matchAll(/<script[^>]+src="([^"]+\.js)"/g)].map((m) => basename(m[1]));
if (entries.length === 0) {
  console.error("No <script type=module> entry found in dist/index.html.");
  process.exit(2);
}

const sizes = new Map();
for (const f of readdirSync(assetsDir)) {
  if (f.endsWith(".js")) sizes.set(f, statSync(join(assetsDir, f)).size);
}

// Static specifiers only. `import(...)` is deliberately not matched: a dynamic edge is the whole
// point of code splitting and must not mark its target eager.
const STATIC_EDGE = /(?:^|[;\n}])\s*(?:import|export)(?:[\s\S]*?from)?\s*["'](\.[^"']+\.js)["']/g;

const eager = new Set();
const queue = [...entries];
while (queue.length > 0) {
  const file = queue.pop();
  if (eager.has(file) || !sizes.has(file)) continue;
  eager.add(file);
  for (const m of readFileSync(join(assetsDir, file), "utf8").matchAll(STATIC_EDGE)) {
    const dep = basename(m[1]);
    if (!eager.has(dep)) queue.push(dep);
  }
}

// Cross-check: Vite emits a modulepreload link for every eagerly-reachable chunk except the entry
// itself, so the two sets must agree. A mismatch means this walker's edge regex has gone stale
// against the emitted output, which would silently under-report the budget.
const preloaded = new Set([
  ...entries,
  ...[...html.matchAll(/<link[^>]+href="([^"]+\.js)"[^>]*>/g)].map((m) => basename(m[1])),
]);
const missed = [...preloaded].filter((f) => !eager.has(f));
const extra = [...eager].filter((f) => !preloaded.has(f));
if (missed.length > 0 || extra.length > 0) {
  console.error(
    `This check disagrees with the modulepreload links Vite emitted, so its numbers cannot be ` +
      `trusted. Update the static-edge regex in this script.\n` +
      `  preloaded but not walked: ${missed.join(", ") || "(none)"}\n` +
      `  walked but not preloaded: ${extra.join(", ") || "(none)"}`,
  );
  process.exit(2);
}

const total = [...eager].reduce((sum, f) => sum + sizes.get(f), 0);
const kB = (n) => `${(n / 1024).toFixed(1)} kB`;

if (total > MAX_EAGER_BYTES) {
  const sorted = [...eager].sort((a, b) => sizes.get(b) - sizes.get(a));
  console.error(
    `Eager bundle budget exceeded: ${kB(total)} across ${eager.size} chunks, ` +
      `over the ${kB(MAX_EAGER_BYTES)} budget by ${kB(total - MAX_EAGER_BYTES)}.\n\n` +
      `A chunk is eager because a STATIC import edge reaches it from the entry. Find the new edge ` +
      `and make it dynamic, or import a concrete module instead of a barrel that re-exports a lazy ` +
      `one (see "Module Graph and Lazy Loading" in README.md). Re-partitioning with ` +
      `\`manualChunks\` does not help - it moves code between chunks without changing what is ` +
      `fetched.\n\n` +
      `If the growth is intentional, raise MAX_EAGER_BYTES in this file in the same commit and say ` +
      `why in the PR description.\n\nLargest eager chunks:\n` +
      sorted
        .slice(0, 10)
        .map((f) => `  ${kB(sizes.get(f)).padStart(9)}  ${f}`)
        .join("\n"),
  );
  process.exit(1);
}

console.log(
  `Eager bundle: ${kB(total)} across ${eager.size} of ${sizes.size} chunks ` +
    `(budget ${kB(MAX_EAGER_BYTES)}, ${kB(MAX_EAGER_BYTES - total)} headroom).`,
);
