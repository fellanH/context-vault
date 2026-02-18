#!/usr/bin/env node

/**
 * postinstall.js — Auto-rebuild native modules on install
 *
 * Detects NODE_MODULE_VERSION mismatches and attempts a rebuild.
 */

import { execSync } from "node:child_process";

async function main() {
  let needsRebuild = false;

  try {
    await import("better-sqlite3");
  } catch (e) {
    if (e.message?.includes("NODE_MODULE_VERSION")) {
      needsRebuild = true;
    }
  }

  try {
    await import("sqlite-vec");
  } catch (e) {
    if (e.message?.includes("NODE_MODULE_VERSION")) {
      needsRebuild = true;
    }
  }

  if (needsRebuild) {
    console.log("[context-vault] Rebuilding native modules for Node.js " + process.version + "...");
    try {
      execSync("npm rebuild better-sqlite3 sqlite-vec", {
        stdio: "inherit",
        timeout: 60000,
      });
      console.log("[context-vault] Native modules rebuilt successfully.");
    } catch {
      console.error("[context-vault] Warning: native module rebuild failed.");
      console.error("[context-vault] Try manually: npm rebuild better-sqlite3 sqlite-vec");
    }
  }
}

main().catch(() => {});
