import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { it, expect } from "vitest";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const barrel = readFileSync(join(__dirname, "index.ts"), "utf8");

it("does not re-export the lazily-imported ContentInputWidget", () => {
  expect(barrel).not.toMatch(/export \{[^}]*\bContentInputWidget\b/);
});
