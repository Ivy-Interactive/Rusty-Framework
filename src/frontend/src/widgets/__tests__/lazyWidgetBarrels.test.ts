import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, it } from "vitest";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const entry = join(srcDir, "index.tsx");
const widgetMapPath = join(srcDir, "widgets", "widgetMap.ts");
const rel = (p: string) => relative(srcDir, p).split(sep).join("/");

function walk(dir: string, out: string[] = []): string[] {
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) walk(full, out);
    else if (/\.tsx?$/.test(name)) out.push(full);
  }
  return out;
}
/** Resolve a first-party specifier ("@/..." or relative) to the file it loads. */
function resolveFrom(fromFile: string, specifier: string): string | null {
  if (!specifier.startsWith("@/") && !specifier.startsWith(".")) return null;
  const base = specifier.startsWith("@/")
    ? join(srcDir, specifier.slice(2))
    : join(dirname(fromFile), specifier);
  const candidates = [`${base}.ts`, `${base}.tsx`, join(base, "index.ts"), join(base, "index.tsx")];
  return candidates.find(existsSync) ?? null;
}

type Edge = { specifier: string; target: string };
const staticEdges = new Map<string, Edge[]>();
const files = walk(srcDir).filter((f) => !/\.test\.tsx?$/.test(f));
for (const file of files) {
  const src = readFileSync(file, "utf8");
  const edges: Edge[] = [];
  // Static `import ... from "x"` / `export ... from "x"`, plus the bare side-effect form
  // `import "x"`. `import type` / `export type` are excluded: they are erased at compile
  // time, so they create no runtime edge.
  for (const m of src.matchAll(
    /(?:^|\n)\s*(?:import|export)\s+(?:(type)\s+[^;\n]*?from|[^;\n]*?from|)\s*"([^"]+)"/g,
  )) {
    if (m[1]) continue;
    const target = resolveFrom(file, m[2]);
    if (target) edges.push({ specifier: m[2], target });
  }
  staticEdges.set(file, edges);
}

/** Drop whole-line comments. `import("...")` is matched anywhere on a line, unlike the
 * static form which is anchored to the start, so widgetMap.ts's own doc comments would
 * otherwise contribute specifiers - including the `@/widgets/<dir>/<Widget>` placeholder. */
const stripComments = (src: string) =>
  src
    .split("\n")
    .filter((line) => !/^\s*(\/\/|\/\*|\*)/.test(line))
    .join("\n");

/** Modules widgetMap.ts expects to be code-split: resolved file -> its specifier. */
const lazyModules = new Map<string, string>();
const dynamicSpecifiers = [
  ...stripComments(readFileSync(widgetMapPath, "utf8")).matchAll(/import\("(@\/[^"]+)"\)/g),
].map((m) => m[1]);
for (const specifier of dynamicSpecifiers) {
  const target = resolveFrom(widgetMapPath, specifier);
  if (target) lazyModules.set(target, specifier);
}

it("resolves every lazy widget module named in widgetMap.ts", () => {
  const unresolved = dynamicSpecifiers.filter((s) => !resolveFrom(widgetMapPath, s));
  expect(unresolved, "unresolvable dynamic import() specifier(s) in widgetMap.ts").toEqual([]);
  expect(lazyModules.size).toBeGreaterThan(40);
});

it("keeps every lazy widget module out of the eager module graph", () => {
  // Walk static edges from the app entry. A lazy module reached this way has been pulled
  // into the eager graph, and its import() no longer splits anything.
  const paths = new Map<string, string[]>([[entry, [rel(entry)]]]);
  const queue = [entry];
  const offenders: string[] = [];
  while (queue.length) {
    const file = queue.shift()!;
    const path = paths.get(file)!;
    for (const { target } of staticEdges.get(file) ?? []) {
      if (paths.has(target)) continue;
      const next = [...path, rel(target)];
      paths.set(target, next);
      if (lazyModules.has(target))
        offenders.push(`${lazyModules.get(target)} is eagerly reachable: ${next.join(" -> ")}`);
      else queue.push(target);
    }
  }
  // Guard against the assertion going vacuous: if the walk stops short of widgetMap.ts
  // (a renamed entry, a moved file, a regex that stops matching) nothing above can fail.
  // This is the load-bearing anti-vacuity check; a stubbed entry reaches 1 file and fails here.
  expect(
    paths.has(widgetMapPath),
    `the eager walk from ${rel(entry)} never reached widgets/widgetMap.ts`,
  ).toBe(true);
  // Deliberately loose. The eager graph is *meant* to shrink as widgets are lazified: it was 199
  // files before 55 widgets moved to import() and is 53 now, so a tight floor here would fail the
  // next lazification rather than a broken walk. Keep it well under the current figure.
  expect(paths.size).toBeGreaterThan(25);
  expect(offenders, `\n${offenders.join("\n")}\n`).toEqual([]);
});
