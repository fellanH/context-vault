/**
 * Multi-step Vite build for Chrome extension.
 *
 * Runs 4 separate builds to avoid Rollup code-splitting issues:
 * 1. Popup — React HTML app (outputs to dist/popup/)
 * 2. Onboarding — React HTML app (outputs to dist/onboarding/)
 * 3. Background — ESM service worker (single file, dist/background.js)
 * 4. Content — IIFE content script (single file, dist/content.js)
 *
 * Then copies static assets (manifest.json, icons/, public/).
 */

import { build } from "vite";
import react from "@vitejs/plugin-react";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { cpSync, existsSync, mkdirSync, rmSync } from "node:fs";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, "..");
const dist = resolve(root, "dist");

const alias = { "@": resolve(root, "src") };

// Clean dist
if (existsSync(dist)) rmSync(dist, { recursive: true });
mkdirSync(dist, { recursive: true });

// ─── 1. Popup (React HTML app) ──────────────────────────────────────────────

console.log("\n[build] Step 1/4 — Popup (React HTML app)");
await build({
  root: resolve(root, "src/popup"),
  plugins: [react()],
  build: {
    outDir: resolve(dist, "popup"),
    emptyOutDir: true,
    rollupOptions: {
      input: resolve(root, "src/popup/index.html"),
    },
    target: "esnext",
    minify: false,
  },
  resolve: { alias },
  logLevel: "warn",
});

// ─── 2. Onboarding (React HTML app) ─────────────────────────────────────────

console.log("[build] Step 2/4 — Onboarding (React HTML app)");
await build({
  root: resolve(root, "src/onboarding"),
  plugins: [react()],
  build: {
    outDir: resolve(dist, "onboarding"),
    emptyOutDir: false,
    rollupOptions: {
      input: resolve(root, "src/onboarding/index.html"),
    },
    target: "esnext",
    minify: false,
  },
  resolve: { alias },
  logLevel: "warn",
});

// ─── 3. Background (ESM service worker) ─────────────────────────────────────

console.log("[build] Step 3/4 — Background (ESM service worker)");
await build({
  root,
  build: {
    outDir: dist,
    emptyOutDir: false,
    lib: {
      entry: resolve(root, "src/background/index.ts"),
      formats: ["es"],
      fileName: () => "background.js",
    },
    rollupOptions: {
      output: { inlineDynamicImports: true },
    },
    target: "esnext",
    minify: false,
  },
  resolve: { alias },
  logLevel: "warn",
});

// ─── 4. Content script (IIFE) ───────────────────────────────────────────────

console.log("[build] Step 4/4 — Content script (IIFE)");
await build({
  root,
  build: {
    outDir: dist,
    emptyOutDir: false,
    lib: {
      entry: resolve(root, "src/content/index.ts"),
      formats: ["iife"],
      fileName: () => "content.js",
      name: "ContextVaultContent",
    },
    rollupOptions: {
      output: { inlineDynamicImports: true },
    },
    target: "esnext",
    minify: false,
  },
  resolve: { alias },
  logLevel: "warn",
});

// ─── 5. Copy static assets ──────────────────────────────────────────────────

console.log("[build] Copying static assets...");

// manifest.json
cpSync(resolve(root, "manifest.json"), resolve(dist, "manifest.json"));

// icons/ (if exists)
if (existsSync(resolve(root, "icons"))) {
  cpSync(resolve(root, "icons"), resolve(dist, "icons"), { recursive: true });
}

// public/ contents (if exists) — privacy.html, etc.
if (existsSync(resolve(root, "public"))) {
  cpSync(resolve(root, "public"), dist, { recursive: true });
}

console.log("[build] Done! Output in dist/\n");
