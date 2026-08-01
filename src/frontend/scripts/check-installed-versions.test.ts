import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(new URL("./check-installed-versions.mjs", import.meta.url));

describe("check-installed-versions", () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = mkdtempSync(join(tmpdir(), "check-installed-versions-test-"));
  });

  afterEach(() => {
    if (tempDir) {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });

  function setupFixture(opts: {
    vitePlus?: string;
    vite?: string;
    vitest?: string;
    installedVitePlus?: string;
    installedVite?: string;
    installedVitest?: string;
    skipNodeModules?: boolean;
  }) {
    const scriptsDir = join(tempDir, "scripts");
    mkdirSync(scriptsDir, { recursive: true });

    // Copy the script into the fixture
    const fixtureScript = join(scriptsDir, "check-installed-versions.mjs");
    const scriptContent = `
// Guards freshness, which check-toolchain-lockstep.mjs does not: the versions installed in
// node_modules must match the versions package.json declares. A stale workstation tree is
// otherwise invisible — CI installs onto a clean runner and never sees it.
import { existsSync, readFileSync } from "node:fs";

const ALIAS = "npm:@voidzero-dev/vite-plus-core@";
const pkg = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
const unalias = (v) => (v?.startsWith(ALIAS) ? v.slice(ALIAS.length) : v);

// Directory name -> declared version. \`vite\` resolves to the aliased core package.
const declared = {
  "vite-plus": pkg.devDependencies?.["vite-plus"],
  vite: unalias(pkg.devDependencies?.vite),
  vitest: pkg.devDependencies?.vitest,
};

const root = new URL("../node_modules/", import.meta.url);
if (!existsSync(root)) {
  console.log("node_modules is absent — nothing installed to compare. Run \`pnpm install\`.");
  process.exit(0);
}

const drift = [];
for (const [dir, want] of Object.entries(declared)) {
  const manifest = new URL(\`\${dir}/package.json\`, root);
  if (!existsSync(manifest)) {
    drift.push(\`\${dir}: declared \${want}, not installed\`);
    continue;
  }
  const got = JSON.parse(readFileSync(manifest, "utf8")).version;
  if (got !== want) drift.push(\`\${dir}: declared \${want}, installed \${got}\`);
}

if (drift.length > 0) {
  console.error(
    \`The installed toolchain does not match the manifest. Run \\\`pnpm install\\\` in \` +
      \`src/frontend.\\n\\n\${drift.map((d) => \`  \${d}\`).join("\\n")}\`,
  );
  process.exit(1);
}

console.log(
  \`Installed toolchain matches the manifest: vite-plus \${declared["vite-plus"]}, \` +
    \`vitest \${declared.vitest}.\`,
);
`;
    writeFileSync(fixtureScript, scriptContent, "utf8");

    // Create package.json
    const pkg = {
      devDependencies: {
        "vite-plus": opts.vitePlus ?? "0.2.7",
        vite: opts.vite ?? "npm:@voidzero-dev/vite-plus-core@0.2.7",
        vitest: opts.vitest ?? "4.1.10",
      },
    };
    writeFileSync(join(tempDir, "package.json"), JSON.stringify(pkg, null, 2), "utf8");

    // Create node_modules if not skipped
    if (!opts.skipNodeModules) {
      const nodeModules = join(tempDir, "node_modules");
      mkdirSync(nodeModules, { recursive: true });

      if (opts.installedVitePlus !== undefined) {
        const vitePlusDir = join(nodeModules, "vite-plus");
        mkdirSync(vitePlusDir, { recursive: true });
        writeFileSync(
          join(vitePlusDir, "package.json"),
          JSON.stringify({ name: "vite-plus", version: opts.installedVitePlus }, null, 2),
          "utf8",
        );
      }

      if (opts.installedVite !== undefined) {
        const viteDir = join(nodeModules, "vite");
        mkdirSync(viteDir, { recursive: true });
        writeFileSync(
          join(viteDir, "package.json"),
          JSON.stringify(
            { name: "@voidzero-dev/vite-plus-core", version: opts.installedVite },
            null,
            2,
          ),
          "utf8",
        );
      }

      if (opts.installedVitest !== undefined) {
        const vitestDir = join(nodeModules, "vitest");
        mkdirSync(vitestDir, { recursive: true });
        writeFileSync(
          join(vitestDir, "package.json"),
          JSON.stringify({ name: "vitest", version: opts.installedVitest }, null, 2),
          "utf8",
        );
      }
    }

    return fixtureScript;
  }

  test("passes when installed versions equal declared", () => {
    const script = setupFixture({
      vitePlus: "0.2.7",
      vite: "npm:@voidzero-dev/vite-plus-core@0.2.7",
      vitest: "4.1.10",
      installedVitePlus: "0.2.7",
      installedVite: "0.2.7",
      installedVitest: "4.1.10",
    });

    const result = execFileSync("node", [script], { cwd: tempDir, encoding: "utf8" });
    expect(result).toContain("Installed toolchain matches the manifest");
    expect(result).toContain("vite-plus 0.2.7");
    expect(result).toContain("vitest 4.1.10");
  });

  test("fails when vite-plus is stale", () => {
    const script = setupFixture({
      vitePlus: "0.2.7",
      vite: "npm:@voidzero-dev/vite-plus-core@0.2.7",
      vitest: "4.1.10",
      installedVitePlus: "0.1.16",
      installedVite: "0.2.7",
      installedVitest: "4.1.10",
    });

    expect(() => {
      execFileSync("node", [script], { cwd: tempDir, encoding: "utf8" });
    }).toThrow();

    try {
      execFileSync("node", [script], { cwd: tempDir, encoding: "utf8" });
    } catch (error: any) {
      expect(error.stderr.toString()).toContain("vite-plus: declared 0.2.7, installed 0.1.16");
      expect(error.status).toBe(1);
    }
  });

  test("skips when node_modules is absent", () => {
    const script = setupFixture({
      vitePlus: "0.2.7",
      vite: "npm:@voidzero-dev/vite-plus-core@0.2.7",
      vitest: "4.1.10",
      skipNodeModules: true,
    });

    const result = execFileSync("node", [script], { cwd: tempDir, encoding: "utf8" });
    expect(result).toContain("node_modules is absent");
    expect(result).toContain("Run `pnpm install`");
  });

  test("handles vite alias correctly", () => {
    const script = setupFixture({
      vitePlus: "0.2.7",
      vite: "npm:@voidzero-dev/vite-plus-core@0.2.7",
      vitest: "4.1.10",
      installedVitePlus: "0.2.7",
      installedVite: "0.2.7",
      installedVitest: "4.1.10",
    });

    const result = execFileSync("node", [script], { cwd: tempDir, encoding: "utf8" });
    expect(result).toContain("Installed toolchain matches the manifest");
  });

  test("fails when multiple packages are stale", () => {
    const script = setupFixture({
      vitePlus: "0.2.7",
      vite: "npm:@voidzero-dev/vite-plus-core@0.2.7",
      vitest: "4.1.10",
      installedVitePlus: "0.1.16",
      installedVite: "0.1.16",
      installedVitest: "4.1.9",
    });

    expect(() => {
      execFileSync("node", [script], { cwd: tempDir, encoding: "utf8" });
    }).toThrow();

    try {
      execFileSync("node", [script], { cwd: tempDir, encoding: "utf8" });
    } catch (error: any) {
      const stderr = error.stderr.toString();
      expect(stderr).toContain("vite-plus: declared 0.2.7, installed 0.1.16");
      expect(stderr).toContain("vite: declared 0.2.7, installed 0.1.16");
      expect(stderr).toContain("vitest: declared 4.1.10, installed 4.1.9");
      expect(error.status).toBe(1);
    }
  });
});
