#!/usr/bin/env node
// Thin launcher — the Rust `context-vault` binary is the whole product (minimal core).
//
// Every command delegates to the native binary (resolved from the per-platform
// optionalDependency, or a locally built target/ binary). No vault logic, no
// TypeScript engine. The Rust CLI provides the MCP server, the engine commands, and
// `setup` (MCP-client registration).

import { runRustEngine } from './rust-launcher.mjs';

process.exit(runRustEngine(process.argv.slice(2)));
