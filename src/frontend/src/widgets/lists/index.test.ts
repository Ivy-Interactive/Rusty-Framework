import { readFileSync } from "node:fs";
import { it, expect } from "vitest";

const barrel = readFileSync(new URL("./index.ts", import.meta.url), "utf8");

it("does not re-export the lazily-imported ListWidget", () => {
  expect(barrel).not.toMatch(/ListWidget["']/);
});
