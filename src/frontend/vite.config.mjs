import path from "path";
import { defineConfig } from "vite-plus";
import mkcert from "vite-plugin-mkcert";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

import { fileURLToPath } from "url";
import { dirname } from "path";
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

function transferMeta(htmlServer, htmlLocal) {
  const titleMatch = htmlServer.match(/<title[^>]*>(.*?)<\/title>/i);
  const serverTitle = titleMatch ? titleMatch[1] : null;

  let result = htmlLocal;

  if (serverTitle) {
    result = result.replace(/<title[^>]*>.*?<\/title>/i, `<title>${serverTitle}</title>`);
  }

  // Transfer ivy-* meta tags
  const ivyMetaMatches = htmlServer.match(/<meta[^>]*name\s*=\s*["']ivy-[^"']*["'][^>]*>/gi);

  // Transfer ivy-custom-theme style tag
  const themeStyleMatch = htmlServer.match(/<style id="ivy-custom-theme">[\s\S]*?<\/style>/i);

  if (ivyMetaMatches || themeStyleMatch) {
    const headEndIndex = result.indexOf("</head>");
    if (headEndIndex !== -1) {
      let toInsert = "";

      if (ivyMetaMatches) {
        toInsert += ivyMetaMatches.map((meta) => ` ${meta}`).join("\n");
      }

      if (themeStyleMatch) {
        if (toInsert) toInsert += "\n";
        toInsert += ` ${themeStyleMatch[0]}`;
      }

      result = result.slice(0, headEndIndex) + toInsert + "\n " + result.slice(headEndIndex);
    }
  }

  return result;
}

function isLocalHost(urlString) {
  try {
    const url = new URL(urlString);
    return ["localhost", "127.0.0.1", "::1"].includes(url.hostname);
  } catch {
    return false;
  }
}

async function fetchText(url) {
  const mod = url.startsWith("https") ? await import("node:https") : await import("node:http");
  const options = isLocalHost(url) ? { rejectUnauthorized: false } : {};
  return new Promise((resolve, reject) => {
    mod
      .get(url, options, (res) => {
        let data = "";
        res.on("data", (chunk) => (data += chunk));
        res.on("end", () => resolve(data));
      })
      .on("error", reject);
  });
}

const injectMeta = (mode) => {
  return {
    name: "inject-ivy-meta",
    async transformIndexHtml(localHtml) {
      if (mode === "development") {
        const host = process.env.IVY_HOST || "https://localhost:5010";
        const serverHtml = await fetchText(`${host}`);
        const transformedHtml = transferMeta(serverHtml, localHtml);
        const ivyHostTag = `<meta name="ivy-host" content="${host}" />`;
        return transformedHtml.replace("</head>", ` ${ivyHostTag}\n</head>`);
      }
      return localHtml;
    },
  };
};

const mode = process.env.NODE_ENV || "development";

/**
 * Fails the build when a dynamically imported first-party module lands in a chunk that is also
 * statically imported - the code-split is defeated and the module ships in the eager graph.
 *
 * This replaces a plugin that promoted Rolldown's INEFFECTIVE_DYNAMIC_IMPORT warning: that warning
 * is never emitted on vite-plus 0.2.7, so the gate was silent while both known bug shapes built
 * with exit 0. Reading the module graph works on the pinned version.
 *
 * node_modules targets are excluded on purpose: five vendor chunks (incl. the deliberate
 * `vendor-markdown` manualChunks target) are legitimately both statically and dynamically
 * imported, so including them would fail a correct build.
 */
const assertLazyChunks = {
  name: "assert-lazy-chunks",
  generateBundle(_options, bundle) {
    const dynamicTargets = new Set();
    for (const id of this.getModuleIds()) {
      const info = this.getModuleInfo(id);
      for (const target of info?.dynamicallyImportedIds ?? []) {
        if (!target.includes("node_modules")) dynamicTargets.add(target);
      }
    }

    const chunkOfModule = new Map();
    const staticallyImported = new Set();
    for (const [name, chunk] of Object.entries(bundle)) {
      if (chunk.type !== "chunk") continue;
      for (const moduleId of chunk.moduleIds ?? []) chunkOfModule.set(moduleId, name);
      for (const imported of chunk.imports) staticallyImported.add(imported);
    }

    const violations = [];
    for (const target of dynamicTargets) {
      const chunk = chunkOfModule.get(target);
      if (chunk && staticallyImported.has(chunk)) {
        violations.push(`  - ${path.relative(__dirname, target).replace(/\\/g, "/")} -> ${chunk}`);
      }
    }

    if (violations.length > 0) {
      this.error(
        `${violations.length} lazily imported module(s) are also statically imported, so they ` +
          `will NOT be code-split:\n${violations.join("\n")}\n\n` +
          `Fix: if the eager exports live in a SIBLING file, import that file directly instead of ` +
          `the barrel. If they live in the SAME file as the lazy export, split the file. ` +
          `See "Module Graph and Lazy Loading" in README.md.`,
      );
    }
  },
};

/**
 * Root package name for a resolved module (handles pnpm nested `node_modules`).
 * @param {string} id
 * @returns {string | null}
 */
function getRootPackageName(id) {
  const parts = id.replace(/\\/g, "/").split("/");
  const nm = parts.lastIndexOf("node_modules");
  if (nm === -1 || nm >= parts.length - 1) return null;
  const a = parts[nm + 1];
  if (a?.startsWith("@")) {
    const b = parts[nm + 2];
    return b ? `${a}/${b}` : a;
  }
  return a ?? null;
}

/** Unified / remark / rehype / KaTeX — must stay in ONE chunk (cross-package circular deps → TDZ in prod). */
const MARKDOWN_STACK_EXACT = new Set([
  "bail",
  "ccount",
  "character-entities",
  "character-reference-invalid",
  "comma-separated-tokens",
  "decode-named-character-reference",
  "devlop",
  "escape-string-regexp",
  "extend",
  "hastscript",
  "html-void-elements",
  "is-alphanumerical",
  "is-alphabetical",
  "is-decimal",
  "is-hexadecimal",
  "is-plain-obj",
  "katex",
  "longest-streak",
  "markdown-table",
  "mdast",
  "property-information",
  "react-markdown",
  "rehype",
  "remark",
  "space-separated-tokens",
  "trim-lines",
  "trough",
  "unified",
  "vfile",
  "vfile-message",
  "zwitch",
]);

/**
 * @param {string | null} pkg
 * @returns {boolean}
 */
function isMarkdownStackPackage(pkg) {
  if (!pkg) return false;
  if (MARKDOWN_STACK_EXACT.has(pkg)) return true;
  return (
    pkg.startsWith("estree-util-") ||
    pkg.startsWith("hast-") ||
    pkg.startsWith("mdast-") ||
    pkg.startsWith("micromark") ||
    pkg.startsWith("rehype-") ||
    pkg.startsWith("remark-") ||
    pkg.startsWith("unist-")
  );
}

/**
 * @param {string} id
 * @returns {string | undefined}
 */
function manualChunks(id) {
  if (!id.includes("node_modules")) return;

  const pkg = getRootPackageName(id);
  if (!pkg) return;

  // 1) Markdown / math pipeline (before react — do not split this graph across chunks)
  if (isMarkdownStackPackage(pkg)) {
    return "vendor-markdown";
  }

  // 2) React core (paths are …/node_modules/react/… not react-markdown)
  if (pkg === "react" || pkg === "react-dom" || pkg === "scheduler") {
    return "vendor-react";
  }

  // 3) Other stable vendor boundaries (loosely coupled to the rest of the app)
  if (pkg === "@microsoft/signalr") {
    return "vendor-signalr";
  }

  return undefined;
}

export default defineConfig({
  base: "./",
  plugins: [react(), tailwindcss(), mkcert(), injectMeta(mode), assertLazyChunks],
  server: {
    proxy: {
      "^/(.*\\.md|llms\\.txt)$": {
        target: process.env.IVY_HOST || "https://localhost:5010",
        changeOrigin: true,
        secure: false,
      },
    },
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  build: {
    target: "es2020",
    outDir: "dist",
    assetsDir: "assets",
    cssCodeSplit: true,
    sourcemap: false,
    rollupOptions: {
      output: {
        entryFileNames: "assets/[name]-[hash].js",
        chunkFileNames: "assets/[name]-[hash].js",
        assetFileNames: "assets/[name]-[hash].[ext]",
        manualChunks,
      },
    },
  },
  test: {
    include: ["**/*.test.ts", "**/*.test.tsx"],
    exclude: ["**/e2e/**", "**/node_modules/**", "**/dist/**"],
    environment: "happy-dom",
  },
});
