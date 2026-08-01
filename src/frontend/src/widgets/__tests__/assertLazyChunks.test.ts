import { describe, it, expect } from "vitest";

/**
 * Tests for the assert-lazy-chunks gate logic from vite.config.mjs.
 *
 * The gate detects when dynamically imported first-party modules are merged into chunks
 * that are part of the eager graph, defeating code-splitting. This test replicates the
 * predicate logic over fixture data to prove it catches both shapes:
 * 1. A lazy module sharing a chunk with something that is statically imported
 * 2. A lazy module merged into the entry chunk
 */

interface MockChunk {
  type: "chunk" | "asset";
  moduleIds?: string[];
  imports?: string[];
  isEntry?: boolean;
}

interface MockBundle {
  [name: string]: MockChunk;
}

interface MockModuleInfo {
  dynamicallyImportedIds?: string[];
}

/**
 * Simulates the gate's violation detection logic.
 * Returns an array of violation messages (empty if no violations).
 */
function detectViolations(moduleInfos: Map<string, MockModuleInfo>, bundle: MockBundle): string[] {
  const dynamicTargets = new Set<string>();
  for (const [_id, info] of moduleInfos.entries()) {
    for (const target of info?.dynamicallyImportedIds ?? []) {
      if (!target.includes("node_modules")) dynamicTargets.add(target);
    }
  }

  const chunkOfModule = new Map<string, string>();
  const staticallyImported = new Set<string>();
  for (const [name, chunk] of Object.entries(bundle)) {
    if (chunk.type !== "chunk") continue;
    for (const moduleId of chunk.moduleIds ?? []) chunkOfModule.set(moduleId, name);
    for (const imported of chunk.imports ?? []) staticallyImported.add(imported);
  }

  const violations: string[] = [];
  for (const target of dynamicTargets) {
    const chunk = chunkOfModule.get(target);
    if (chunk && (staticallyImported.has(chunk) || bundle[chunk]?.isEntry)) {
      violations.push(`${target} -> ${chunk}`);
    }
  }

  return violations;
}

describe("assertLazyChunks gate logic", () => {
  it("detects a lazy module in a chunk that is statically imported by another chunk", () => {
    const moduleInfos = new Map<string, MockModuleInfo>([
      ["src/entry.ts", { dynamicallyImportedIds: ["src/lazy.ts"] }],
    ]);

    const bundle: MockBundle = {
      "chunkA.js": {
        type: "chunk",
        moduleIds: ["src/lazy.ts"],
        imports: [],
        isEntry: false,
      },
      "chunkB.js": {
        type: "chunk",
        moduleIds: ["src/other.ts"],
        imports: ["chunkA.js"], // chunkB statically imports chunkA
        isEntry: false,
      },
    };

    const violations = detectViolations(moduleInfos, bundle);
    expect(violations).toHaveLength(1);
    expect(violations[0]).toContain("src/lazy.ts");
    expect(violations[0]).toContain("chunkA.js");
  });

  it("detects a lazy module merged into the entry chunk", () => {
    const moduleInfos = new Map<string, MockModuleInfo>([
      ["src/entry.ts", { dynamicallyImportedIds: ["src/lazy.ts"] }],
    ]);

    const bundle: MockBundle = {
      "entry.js": {
        type: "chunk",
        moduleIds: ["src/entry.ts", "src/lazy.ts"], // lazy merged into entry
        imports: [],
        isEntry: true,
      },
    };

    const violations = detectViolations(moduleInfos, bundle);
    expect(violations).toHaveLength(1);
    expect(violations[0]).toContain("src/lazy.ts");
    expect(violations[0]).toContain("entry.js");
  });

  it("does not flag a genuinely lazy chunk", () => {
    const moduleInfos = new Map<string, MockModuleInfo>([
      ["src/entry.ts", { dynamicallyImportedIds: ["src/lazy.ts"] }],
    ]);

    const bundle: MockBundle = {
      "entry.js": {
        type: "chunk",
        moduleIds: ["src/entry.ts"],
        imports: [],
        isEntry: true,
      },
      "lazy.js": {
        type: "chunk",
        moduleIds: ["src/lazy.ts"],
        imports: [],
        isEntry: false,
      },
    };

    const violations = detectViolations(moduleInfos, bundle);
    expect(violations).toHaveLength(0);
  });

  it("excludes node_modules from violation detection", () => {
    const moduleInfos = new Map<string, MockModuleInfo>([
      ["src/entry.ts", { dynamicallyImportedIds: ["node_modules/vendor/lib.js"] }],
    ]);

    const bundle: MockBundle = {
      "entry.js": {
        type: "chunk",
        moduleIds: ["src/entry.ts", "node_modules/vendor/lib.js"],
        imports: [],
        isEntry: true,
      },
    };

    const violations = detectViolations(moduleInfos, bundle);
    expect(violations).toHaveLength(0);
  });
});
