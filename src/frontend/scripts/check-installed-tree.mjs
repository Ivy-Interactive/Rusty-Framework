// Fails when node_modules disagrees with what package.json declares for the Vite+ toolchain.
// Companion to check-toolchain-lockstep.mjs, which only compares manifest entries to each other
// and so reports green on a stale installed tree.
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

const ALIAS = "npm:@voidzero-dev/vite-plus-core@";
const require = createRequire(import.meta.url);
const pkg = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
const unalias = (v) => (v?.startsWith(ALIAS) ? v.slice(ALIAS.length) : v);

const WATCHED = [
  ["vite-plus", pkg.devDependencies?.["vite-plus"]],
  ["vite", unalias(pkg.devDependencies?.vite)],
  ["vitest", pkg.devDependencies?.vitest],
  ["typescript", pkg.devDependencies?.typescript],
];

const problems = [];
for (const [name, declared] of WATCHED) {
  // Only exact pins are checkable; a range legitimately resolves to something else.
  if (declared === undefined || /^[\^~><*]/.test(declared)) continue;
  let installed;
  try {
    installed = require(`${name}/package.json`).version;
  } catch {
    problems.push(`${name}: declared ${declared}, but not installed — run \`pnpm install\`.`);
    continue;
  }
  if (installed !== declared) {
    problems.push(`${name}: declared ${declared}, installed ${installed}.`);
  }
}

if (problems.length > 0) {
  console.error(
    `src/frontend/node_modules disagrees with package.json. Run \`pnpm install\` before ` +
      `trusting any local build, lint or test result.\n\n  ${problems.join("\n  ")}`,
  );
  process.exit(1);
}
console.log(`Installed tree matches the manifest for ${WATCHED.length} pinned toolchain packages.`);
