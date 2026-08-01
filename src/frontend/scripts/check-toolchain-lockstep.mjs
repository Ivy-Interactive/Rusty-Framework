// Guards the invariant documented in AGENTS.md: the five manifest entries carrying the Vite+
// toolchain must all move together, and `vitest` must match the version `vite-plus` pins exactly.
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

const ALIAS = "npm:@voidzero-dev/vite-plus-core@";
const pkg = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));

for (const [key, raw] of [
  ["devDependencies.vite", pkg.devDependencies?.vite],
  ["pnpm.overrides.vite", pkg.pnpm?.overrides?.vite],
]) {
  if (raw !== undefined && !raw.startsWith(ALIAS)) {
    console.error(
      `${key} must alias the Vite+ core package: expected "${ALIAS}<version>", got "${raw}".`,
    );
    process.exit(1);
  }
}

const unalias = (v) => (v?.startsWith(ALIAS) ? v.slice(ALIAS.length) : v);
const vite = {
  "devDependencies.vite": unalias(pkg.devDependencies?.vite),
  "pnpm.overrides.vite": unalias(pkg.pnpm?.overrides?.vite),
  "devDependencies.vite-plus": pkg.devDependencies?.["vite-plus"],
};
const vitest = {
  "devDependencies.vitest": pkg.devDependencies?.vitest,
  "pnpm.overrides.vitest": pkg.pnpm?.overrides?.vitest,
};

const show = (group) =>
  Object.entries(group)
    .map(([key, value]) => `  ${key} = ${value ?? "(missing)"}`)
    .join("\n");
const agreed = (group) => {
  const values = Object.values(group);
  return values.every((v) => v !== undefined) && new Set(values).size === 1 ? values[0] : null;
};

const errors = [];
const vitePlusVersion = agreed(vite);
const vitestVersion = agreed(vitest);

if (!vitePlusVersion) errors.push(`vite-plus versions disagree:\n${show(vite)}`);
if (!vitestVersion) errors.push(`vitest versions disagree:\n${show(vitest)}`);

if (vitePlusVersion && vitestVersion) {
  let pinned = null;
  try {
    pinned = execFileSync("npm", ["view", `vite-plus@${vitePlusVersion}`, "dependencies.vitest"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    }).trim();
  } catch (error) {
    console.warn(
      `WARNING: could not query the npm registry, so the vitest pin of ` +
        `vite-plus@${vitePlusVersion} was NOT verified — the five manifest entries do agree.\n  ` +
        (error.stderr?.toString().trim().split("\n")[0] ?? error.message),
    );
  }
  if (pinned === "") {
    console.warn(
      `WARNING: vite-plus@${vitePlusVersion} declares no vitest dependency, so there is no ` +
        `pin to compare against. Check the manifest by hand.`,
    );
  } else if (pinned && pinned !== vitestVersion) {
    errors.push(
      `vite-plus@${vitePlusVersion} pins vitest ${pinned}, but the manifest declares ${vitestVersion}.`,
    );
  }
}

// Check installed tree against declared versions
const installedChecks = [
  { packageName: "vite-plus", expectedVersion: vitePlusVersion },
  { packageName: "vite", realName: "@voidzero-dev/vite-plus-core", expectedVersion: vitePlusVersion },
  { packageName: "vitest", expectedVersion: vitestVersion },
];

const installed = {};
let anyMissing = false;

for (const { packageName, realName, expectedVersion } of installedChecks) {
  const resolveName = realName ?? packageName;
  try {
    const require = createRequire(new URL("../package.json", import.meta.url));
    const pkgPath = require.resolve(`${resolveName}/package.json`);
    const pkgData = JSON.parse(readFileSync(pkgPath, "utf8"));
    installed[packageName] = pkgData.version;

    if (pkgData.version !== expectedVersion) {
      errors.push(
        `${packageName}: package.json declares ${expectedVersion} but node_modules has ${pkgData.version} — ` +
          `check pnpm.overrides for an entry on this package, then re-run pnpm install.`,
      );
    }
  } catch {
    anyMissing = true;
  }
}

if (errors.length > 0) {
  console.error(
    `The Vite+ toolchain is out of lockstep. All five entries must move together in one ` +
      `change — see AGENTS.md.\n\n${errors.join("\n\n")}`,
  );
  process.exit(1);
}

const installedSummary = anyMissing
  ? "(installed tree not checked — node_modules absent)"
  : `(installed tree matches: vite-plus ${installed["vite-plus"]}, vite ${installed["vite"]}, vitest ${installed["vitest"]})`;

console.log(
  `Vite+ toolchain in lockstep: vite-plus ${vitePlusVersion}, vitest ${vitestVersion}. ${installedSummary}`,
);
