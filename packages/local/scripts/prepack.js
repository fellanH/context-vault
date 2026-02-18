#!/usr/bin/env node

/**
 * prepack.js — Cross-platform bundle preparation
 *
 * Copies @context-vault/core into node_modules for npm pack bundling.
 * Replaces the Unix shell script in package.json "prepack".
 */

import { cpSync, rmSync, mkdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const LOCAL_ROOT = join(__dirname, "..");
const CORE_SRC = join(LOCAL_ROOT, "..", "core");
const CORE_DEST = join(LOCAL_ROOT, "node_modules", "@context-vault", "core");

// Ensure target directory exists
mkdirSync(join(LOCAL_ROOT, "node_modules", "@context-vault"), { recursive: true });

// Remove old copy if present
rmSync(CORE_DEST, { recursive: true, force: true });

// Copy core package (dereference symlinks)
cpSync(CORE_SRC, CORE_DEST, { recursive: true, dereference: true });

// Remove nested node_modules from the copy
rmSync(join(CORE_DEST, "node_modules"), { recursive: true, force: true });

console.log("[prepack] Bundled @context-vault/core into node_modules");
