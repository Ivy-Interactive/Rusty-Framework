// Guards freshness, which check-toolchain-lockstep.mjs does not: the versions installed in
// node_modules must match the versions package.json declares. A stale workstation tree is
// otherwise invisible — CI installs onto a clean runner and never sees it.
import { existsSync, readFileSync } from "node:fs";

const ALIAS = "npm:@voidzero-dev/vite-plus-core@";
const pkg = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
const unalias = (v) => (v?.startsWith(ALIAS) ? v.slice(ALIAS.length) : v);

// Directory name -> declared version. `vite` resolves to the aliased core package.
const declared = {
  "vite-plus": pkg.devDependencies?.["vite-plus"],
  vite: unalias(pkg.devDependencies?.vite),
  vitest: pkg.devDependencies?.vitest,
};

const root = new URL("../node_modules/", import.meta.url);
if (!existsSync(root)) {
  console.log("node_modules is absent — nothing installed to compare. Run `pnpm install`.");
  process.exit(0);
}

const drift = [];
for (const [dir, want] of Object.entries(declared)) {
  const manifest = new URL(`${dir}/package.json`, root);
  if (!existsSync(manifest)) {
    drift.push(`${dir}: declared ${want}, not installed`);
    continue;
  }
  const got = JSON.parse(readFileSync(manifest, "utf8")).version;
  if (got !== want) drift.push(`${dir}: declared ${want}, installed ${got}`);
}

if (drift.length > 0) {
  console.error(
    `The installed toolchain does not match the manifest. Run \`pnpm install\` in ` +
      `src/frontend.\n\n${drift.map((d) => `  ${d}`).join("\n")}`,
  );
  process.exit(1);
}

console.log(
  `Installed toolchain matches the manifest: vite-plus ${declared["vite-plus"]}, ` +
    `vitest ${declared.vitest}.`,
);
