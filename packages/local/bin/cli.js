#!/usr/bin/env node

/**
 * context-mcp CLI — Unified entry point
 *
 * Usage:
 *   context-mcp setup              Interactive MCP installer
 *   context-mcp ui [--port 3141]   Launch web dashboard
 *   context-mcp reindex            Rebuild search index
 *   context-mcp status             Show vault diagnostics
 */

import { createInterface } from "node:readline";
import {
  existsSync,
  readFileSync,
  writeFileSync,
  mkdirSync,
  copyFileSync,
  unlinkSync,
} from "node:fs";
import { join, resolve, dirname } from "node:path";
import { homedir, platform } from "node:os";
import { execSync, fork } from "node:child_process";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = resolve(__dirname, "..");
const HOME = homedir();

const pkg = JSON.parse(readFileSync(join(ROOT, "package.json"), "utf-8"));
const VERSION = pkg.version;
const SERVER_PATH = resolve(ROOT, "src", "server", "index.js");

/** Detect if running as an npm-installed package (global or local) vs local dev clone */
function isInstalledPackage() {
  return ROOT.includes("/node_modules/") || ROOT.includes("\\node_modules\\");
}

// ─── ANSI Helpers ────────────────────────────────────────────────────────────

const bold = (s) => `\x1b[1m${s}\x1b[0m`;
const dim = (s) => `\x1b[2m${s}\x1b[0m`;
const green = (s) => `\x1b[32m${s}\x1b[0m`;
const yellow = (s) => `\x1b[33m${s}\x1b[0m`;
const cyan = (s) => `\x1b[36m${s}\x1b[0m`;
const red = (s) => `\x1b[31m${s}\x1b[0m`;

// ─── Arg Parsing ─────────────────────────────────────────────────────────────

const args = process.argv.slice(2);
const command = args[0];
const flags = new Set(args.filter((a) => a.startsWith("--")));
const isNonInteractive = flags.has("--yes") || !process.stdin.isTTY;

function getFlag(name) {
  const idx = args.indexOf(name);
  return idx !== -1 && args[idx + 1] ? args[idx + 1] : null;
}

// ─── Readline Prompt ─────────────────────────────────────────────────────────

function prompt(question, defaultVal) {
  if (isNonInteractive) return Promise.resolve(defaultVal || "");
  const rl = createInterface({ input: process.stdin, output: process.stdout });
  const suffix = defaultVal ? ` ${dim(`(${defaultVal})`)}` : "";
  return new Promise((res) => {
    rl.question(`${question}${suffix} `, (answer) => {
      rl.close();
      res(answer.trim() || defaultVal || "");
    });
  });
}

// ─── Platform Helpers ────────────────────────────────────────────────────────

const PLATFORM = platform();

/** Get the platform-specific application data directory */
function appDataDir() {
  switch (PLATFORM) {
    case "win32":
      return process.env.APPDATA || join(HOME, "AppData", "Roaming");
    case "darwin":
      return join(HOME, "Library", "Application Support");
    case "linux":
    default:
      return process.env.XDG_CONFIG_HOME || join(HOME, ".config");
  }
}

/** Get the platform-specific VS Code extensions directory */
function vscodeDataDir() {
  switch (PLATFORM) {
    case "win32":
      return join(appDataDir(), "Code", "User", "globalStorage");
    case "darwin":
      return join(appDataDir(), "Code", "User", "globalStorage");
    case "linux":
    default:
      return join(HOME, ".config", "Code", "User", "globalStorage");
  }
}

// ─── Tool Detection ──────────────────────────────────────────────────────────

const TOOLS = [
  {
    id: "claude-code",
    name: "Claude Code",
    detect: () => {
      try {
        const cmd = PLATFORM === "win32" ? "where claude" : "which claude";
        execSync(cmd, { stdio: "pipe" });
        return true;
      } catch {
        return false;
      }
    },
    configType: "cli",
  },
  {
    id: "claude-desktop",
    name: "Claude Desktop",
    detect: () => existsSync(join(appDataDir(), "Claude")),
    configType: "json",
    configPath: join(appDataDir(), "Claude", "claude_desktop_config.json"),
    configKey: "mcpServers",
  },
  {
    id: "cursor",
    name: "Cursor",
    detect: () => existsSync(join(HOME, ".cursor")),
    configType: "json",
    configPath: join(HOME, ".cursor", "mcp.json"),
    configKey: "mcpServers",
  },
  {
    id: "windsurf",
    name: "Windsurf",
    detect: () => existsSync(join(HOME, ".codeium", "windsurf")),
    configType: "json",
    configPath: join(HOME, ".codeium", "windsurf", "mcp_config.json"),
    configKey: "mcpServers",
  },
  {
    id: "cline",
    name: "Cline (VS Code)",
    detect: () =>
      existsSync(join(vscodeDataDir(), "saoudrizwan.claude-dev", "settings")),
    configType: "json",
    configPath: join(
      vscodeDataDir(),
      "saoudrizwan.claude-dev",
      "settings",
      "cline_mcp_settings.json"
    ),
    configKey: "mcpServers",
  },
];

// ─── Help ────────────────────────────────────────────────────────────────────

function showHelp() {
  console.log(`
  ${bold("◇ context-vault")} ${dim(`v${VERSION}`)}
  ${dim("Persistent memory for AI agents")}

${bold("Usage:")}
  context-mcp <command> [options]

${bold("Commands:")}
  ${cyan("setup")}                 Interactive MCP server installer
  ${cyan("serve")}                 Start the MCP server (used by AI clients)
  ${cyan("ui")} [--port 3141]      Launch web dashboard
  ${cyan("reindex")}               Rebuild search index from knowledge files
  ${cyan("status")}                Show vault diagnostics
  ${cyan("update")}                Check for and install updates
  ${cyan("uninstall")}             Remove MCP configs and optionally data
  ${cyan("migrate")}               Migrate vault between local and hosted

${bold("Options:")}
  --help                Show this help
  --version             Show version
  --yes                 Non-interactive mode (accept all defaults)
`);
}

// ─── Setup Command ───────────────────────────────────────────────────────────

async function runSetup() {
  // Banner
  console.log();
  console.log(`  ${bold("◇ context-vault")} ${dim(`v${VERSION}`)}`);
  console.log(dim("  Persistent memory for AI agents"));
  console.log();

  // Check for existing installation
  const existingConfig = join(HOME, ".context-mcp", "config.json");
  if (existsSync(existingConfig) && !isNonInteractive) {
    let existingVault = "(unknown)";
    try {
      const cfg = JSON.parse(readFileSync(existingConfig, "utf-8"));
      existingVault = cfg.vaultDir || existingVault;
    } catch {}

    console.log(yellow(`  Existing installation detected`));
    console.log(dim(`  Vault: ${existingVault}`));
    console.log(dim(`  Config: ${existingConfig}`));
    console.log();
    console.log(`    1) Full reconfigure`);
    console.log(`    2) Update tool configs only ${dim("(skip vault setup)")}`);
    console.log(`    3) Cancel`);
    console.log();
    const choice = await prompt("  Select:", "1");

    if (choice === "3") {
      console.log(dim("  Cancelled."));
      return;
    }

    if (choice === "2") {
      // Skip vault setup, just reconfigure tools
      console.log();
      console.log(dim(`  [1/2]`) + bold(" Detecting tools...\n"));
      const detected = [];
      for (const tool of TOOLS) {
        const found = tool.detect();
        if (found) {
          detected.push(tool);
          console.log(`  ${green("+")} ${tool.name}`);
        } else {
          console.log(`  ${dim("-")} ${dim(tool.name)} ${dim("(not found)")}`);
        }
      }
      console.log();

      if (detected.length === 0) {
        console.log(yellow("  No supported tools detected."));
        return;
      }

      let selected;
      console.log(bold("  Which tools should context-mcp connect to?\n"));
      for (let i = 0; i < detected.length; i++) {
        console.log(`    ${i + 1}) ${detected[i].name}`);
      }
      console.log();
      const answer = await prompt(
        `  Select (${dim("1,2,3")} or ${dim('"all"')}):`,
        "all"
      );
      if (answer === "all" || answer === "") {
        selected = detected;
      } else {
        const nums = answer.split(/[,\s]+/).map((n) => parseInt(n, 10) - 1).filter((n) => n >= 0 && n < detected.length);
        selected = nums.map((n) => detected[n]);
        if (selected.length === 0) selected = detected;
      }

      // Read vault dir from existing config
      let customVaultDir = null;
      try {
        const cfg = JSON.parse(readFileSync(existingConfig, "utf-8"));
        const defaultVDir = join(HOME, "vault");
        if (cfg.vaultDir && resolve(cfg.vaultDir) !== resolve(defaultVDir)) {
          customVaultDir = cfg.vaultDir;
        }
      } catch {}

      console.log(`\n  ${dim("[2/2]")}${bold(" Configuring tools...\n")}`);
      for (const tool of selected) {
        try {
          if (tool.configType === "cli") {
            await configureClaude(tool, customVaultDir);
          } else {
            configureJsonTool(tool, customVaultDir);
          }
          console.log(`  ${green("+")} ${tool.name} — configured`);
        } catch (e) {
          console.log(`  ${red("x")} ${tool.name} — ${e.message}`);
        }
      }

      console.log();
      console.log(green("  ✓ Tool configs updated."));
      console.log();
      return;
    }
    // choice === "1" falls through to full setup below
    console.log();
  }

  // Detect tools
  console.log(dim(`  [1/5]`) + bold(" Detecting tools...\n"));
  const detected = [];
  for (const tool of TOOLS) {
    const found = tool.detect();
    if (found) {
      detected.push(tool);
      console.log(`  ${green("+")} ${tool.name}`);
    } else {
      console.log(`  ${dim("-")} ${dim(tool.name)} ${dim("(not found)")}`);
    }
  }
  console.log();

  if (detected.length === 0) {
    console.log(
      yellow("  No supported tools detected.\n")
    );
    console.log("  To manually configure, add to your tool's MCP config:\n");
    if (isInstalledPackage()) {
      console.log(`  ${dim("{")}
    ${dim('"mcpServers": {')}
      ${dim('"context-mcp": {')}
        ${dim('"command": "context-mcp",')}
        ${dim(`"args": ["serve", "--vault-dir", "/path/to/vault"]`)}
      ${dim("}")}
    ${dim("}")}
  ${dim("}")}\n`);
    } else {
      console.log(`  ${dim("{")}
    ${dim('"mcpServers": {')}
      ${dim('"context-mcp": {')}
        ${dim('"command": "node",')}
        ${dim(`"args": ["${SERVER_PATH}", "--vault-dir", "/path/to/vault"]`)}
      ${dim("}")}
    ${dim("}")}
  ${dim("}")}\n`);
    }
    return;
  }

  // Select tools
  let selected;
  if (isNonInteractive) {
    selected = detected;
  } else {
    console.log(bold("  Which tools should context-mcp connect to?\n"));
    for (let i = 0; i < detected.length; i++) {
      console.log(`    ${i + 1}) ${detected[i].name}`);
    }
    console.log();
    const answer = await prompt(
      `  Select (${dim("1,2,3")} or ${dim('"all"')}):`,
      "all"
    );
    if (answer === "all" || answer === "") {
      selected = detected;
    } else {
      const nums = answer
        .split(/[,\s]+/)
        .map((n) => parseInt(n, 10) - 1)
        .filter((n) => n >= 0 && n < detected.length);
      selected = nums.map((n) => detected[n]);
      if (selected.length === 0) selected = detected;
    }
  }

  // Vault directory (content files)
  console.log(dim(`  [2/5]`) + bold(" Configuring vault...\n"));
  const defaultVaultDir = join(HOME, "vault");
  const vaultDir = isNonInteractive
    ? defaultVaultDir
    : await prompt(`\n  Vault directory:`, defaultVaultDir);
  const resolvedVaultDir = resolve(vaultDir);

  // Create vault dir if needed
  if (!existsSync(resolvedVaultDir)) {
    if (isNonInteractive) {
      mkdirSync(resolvedVaultDir, { recursive: true });
      console.log(
        `\n  ${green("+")} Created ${resolvedVaultDir}`
      );
    } else {
      const create = await prompt(
        `\n  ${resolvedVaultDir} doesn't exist. Create it? (Y/n):`,
        "Y"
      );
      if (create.toLowerCase() !== "n") {
        mkdirSync(resolvedVaultDir, { recursive: true });
        console.log(`  ${green("+")} Created ${resolvedVaultDir}`);
      }
    }
  }

  // Ensure data dir exists for DB storage
  const dataDir = join(HOME, ".context-mcp");
  mkdirSync(dataDir, { recursive: true });

  // Write config.json to data dir (persistent, survives reinstalls)
  const configPath = join(dataDir, "config.json");
  const vaultConfig = {};
  if (existsSync(configPath)) {
    try {
      Object.assign(vaultConfig, JSON.parse(readFileSync(configPath, "utf-8")));
    } catch {}
  }
  vaultConfig.vaultDir = resolvedVaultDir;
  vaultConfig.dataDir = dataDir;
  vaultConfig.dbPath = join(dataDir, "vault.db");
  vaultConfig.devDir = join(HOME, "dev");
  writeFileSync(configPath, JSON.stringify(vaultConfig, null, 2) + "\n");
  console.log(`\n  ${green("+")} Wrote ${configPath}`);

  // Pre-download embedding model
  console.log(`\n  ${dim("[3/5]")}${bold(" Downloading embedding model...")}`);
  console.log(dim("  all-MiniLM-L6-v2 (~22MB, one-time download)\n"));
  try {
    const { embed } = await import("@context-vault/core/index/embed");
    await embed("warmup");
    console.log(`  ${green("+")} Embedding model ready`);
  } catch (e) {
    console.log(`  ${yellow("!")} Model download failed — will retry on first use`);
  }

  // Clean up legacy project-root config.json if it exists
  const legacyConfigPath = join(ROOT, "config.json");
  if (existsSync(legacyConfigPath)) {
    try {
      unlinkSync(legacyConfigPath);
      console.log(`  ${dim("Removed legacy config at " + legacyConfigPath)}`);
    } catch {}
  }

  // Configure each tool — pass vault dir as arg if non-default
  console.log(`\n  ${dim("[4/5]")}${bold(" Configuring tools...\n")}`);
  const results = [];
  const defaultVDir = join(HOME, "vault");
  const customVaultDir = resolvedVaultDir !== resolve(defaultVDir) ? resolvedVaultDir : null;

  for (const tool of selected) {
    try {
      if (tool.configType === "cli") {
        await configureClaude(tool, customVaultDir);
      } else {
        configureJsonTool(tool, customVaultDir);
      }
      results.push({ tool, ok: true });
      console.log(`  ${green("+")} ${tool.name} — configured`);
    } catch (e) {
      results.push({ tool, ok: false, error: e.message });
      console.log(`  ${red("x")} ${tool.name} — ${e.message}`);
    }
  }

  // Seed entry
  const seeded = createSeedEntries(resolvedVaultDir);
  if (seeded > 0) {
    console.log(`\n  ${green("+")} Created ${seeded} starter ${seeded === 1 ? "entry" : "entries"} in vault`);
  }

  // Offer to launch UI
  console.log();
  if (!isNonInteractive) {
    const launchUi = await prompt(
      `  Launch web dashboard? (y/N):`,
      "N"
    );
    if (launchUi.toLowerCase() === "y") {
      console.log();
      runUi();
      return;
    }
  }

  // Health check
  console.log(`\n  ${dim("[5/5]")}${bold(" Health check...")}\n`);
  const okResults = results.filter((r) => r.ok);
  const checks = [
    { label: "Vault directory exists", pass: existsSync(resolvedVaultDir) },
    { label: "Config file written", pass: existsSync(configPath) },
    { label: "At least one tool configured", pass: okResults.length > 0 },
  ];
  const passed = checks.filter((c) => c.pass).length;
  for (const c of checks) {
    console.log(`  ${c.pass ? green("✓") : red("✗")} ${c.label}`);
  }

  // Completion box
  const toolName = okResults.length ? okResults[0].tool.name : "your AI tool";
  const boxLines = [
    `  ✓ Setup complete — ${passed}/${checks.length} checks passed`,
    ``,
    `  ${bold("AI Tools")} — open ${toolName} and try:`,
    `  "Search my vault for getting started"`,
    `  "Save an insight about [topic]"`,
    `  "Show my vault status"`,
    ``,
    `  ${bold("CLI Commands:")}`,
    `  context-mcp status    Show vault health`,
    `  context-mcp ui        Launch web dashboard`,
    `  context-mcp update    Check for updates`,
  ];
  const innerWidth = Math.max(...boxLines.map((l) => l.length)) + 2;
  const pad = (s) => s + " ".repeat(Math.max(0, innerWidth - s.length));
  console.log();
  console.log(`  ${dim("┌" + "─".repeat(innerWidth) + "┐")}`);
  for (const line of boxLines) {
    console.log(`  ${dim("│")}${pad(line)}${dim("│")}`);
  }
  console.log(`  ${dim("└" + "─".repeat(innerWidth) + "┘")}`);
  console.log();
}

async function configureClaude(tool, vaultDir) {
  const env = { ...process.env };
  delete env.CLAUDECODE;

  try {
    execSync("claude mcp remove context-mcp -s user", { stdio: "pipe", env });
  } catch {}

  try {
    execSync("claude mcp remove context-vault -s user", { stdio: "pipe", env });
  } catch {}

  if (isInstalledPackage()) {
    const cmdArgs = ["serve"];
    if (vaultDir) cmdArgs.push("--vault-dir", `"${vaultDir}"`);
    execSync(
      `claude mcp add -s user context-mcp -- context-mcp ${cmdArgs.join(" ")}`,
      { stdio: "pipe", env }
    );
  } else {
    const cmdArgs = [`"${SERVER_PATH}"`];
    if (vaultDir) cmdArgs.push("--vault-dir", `"${vaultDir}"`);
    execSync(
      `claude mcp add -s user context-mcp -- node ${cmdArgs.join(" ")}`,
      { stdio: "pipe", env }
    );
  }
}

function configureJsonTool(tool, vaultDir) {
  const configPath = tool.configPath;
  const configDir = dirname(configPath);

  if (!existsSync(configDir)) {
    mkdirSync(configDir, { recursive: true });
  }

  let config = {};
  if (existsSync(configPath)) {
    const raw = readFileSync(configPath, "utf-8");
    try {
      config = JSON.parse(raw);
    } catch {
      const bakPath = configPath + ".bak";
      copyFileSync(configPath, bakPath);
      console.log(`  ${yellow("!")} Backed up corrupted config to ${bakPath}`);
      config = {};
    }
  }

  if (!config[tool.configKey]) {
    config[tool.configKey] = {};
  }

  delete config[tool.configKey]["context-vault"];

  if (isInstalledPackage()) {
    const serverArgs = ["serve"];
    if (vaultDir) serverArgs.push("--vault-dir", vaultDir);
    config[tool.configKey]["context-mcp"] = {
      command: "context-mcp",
      args: serverArgs,
    };
  } else {
    const serverArgs = [SERVER_PATH];
    if (vaultDir) serverArgs.push("--vault-dir", vaultDir);
    config[tool.configKey]["context-mcp"] = {
      command: "node",
      args: serverArgs,
    };
  }

  writeFileSync(configPath, JSON.stringify(config, null, 2) + "\n");
}

// ─── Seed Entries ────────────────────────────────────────────────────────────

function createSeedEntries(vaultDir) {
  let created = 0;

  // Entry 1: Getting started (improved)
  const insightDir = join(vaultDir, "knowledge", "insights");
  const insightPath = join(insightDir, "getting-started.md");
  if (!existsSync(insightPath)) {
    mkdirSync(insightDir, { recursive: true });
    const id1 = Date.now().toString(36).toUpperCase().padStart(10, "0");
    const now = new Date().toISOString();
    writeFileSync(insightPath, `---
id: ${id1}
tags: ["getting-started", "vault"]
source: context-mcp-setup
created: ${now}
---
Welcome to your context vault! This is a seed entry created during setup.

Your vault stores knowledge as plain markdown files with YAML frontmatter.
AI agents search it using hybrid full-text + semantic search.

**Quick start:**
- "Search my vault for getting started" — find this entry
- "Save an insight about [topic]" — add knowledge
- "Show my vault status" — check health
- "List my recent entries" — browse your vault

You can edit or delete this file anytime — it lives at:
${insightPath}
`);
    created++;
  }

  // Entry 2: Example decision
  const decisionDir = join(vaultDir, "knowledge", "decisions");
  const decisionPath = join(decisionDir, "example-local-first-data.md");
  if (!existsSync(decisionPath)) {
    mkdirSync(decisionDir, { recursive: true });
    const id2 = (Date.now() + 1).toString(36).toUpperCase().padStart(10, "0");
    const now = new Date().toISOString();
    writeFileSync(decisionPath, `---
id: ${id2}
tags: ["example", "architecture"]
source: context-mcp-setup
created: ${now}
---
Example decision: Use local-first data storage (SQLite + files) over cloud databases.

**Context:** For personal knowledge management, local storage provides better privacy,
offline access, and zero ongoing cost. The vault uses plain markdown files as the
source of truth with a SQLite index for fast search.

**Trade-offs:**
- Pro: Full data ownership, git-versioned, human-editable
- Pro: No cloud dependency, works offline
- Con: No built-in sync across devices (use git or Syncthing)

This is an example entry showing the decision format. Feel free to delete it.
`);
    created++;
  }

  return created;
}

// ─── UI Command ──────────────────────────────────────────────────────────────

function runUi() {
  const serveScript = resolve(ROOT, "ui", "serve.js");
  if (!existsSync(serveScript)) {
    console.error(red("Error: ui/serve.js not found."));
    process.exit(1);
  }

  const uiArgs = args.slice(1);
  const child = fork(serveScript, uiArgs, { stdio: "inherit" });
  child.on("exit", (code) => process.exit(code ?? 0));
}

// ─── Reindex Command ─────────────────────────────────────────────────────────

async function runReindex() {
  console.log(dim("Loading vault..."));

  const { resolveConfig } = await import("@context-vault/core/core/config");
  const { initDatabase, prepareStatements, insertVec, deleteVec } = await import("@context-vault/core/index/db");
  const { embed } = await import("@context-vault/core/index/embed");
  const { reindex } = await import("@context-vault/core/index");

  const config = resolveConfig();
  if (!config.vaultDirExists) {
    console.error(
      red(`Vault directory not found: ${config.vaultDir}`)
    );
    console.error("Run " + cyan("context-mcp setup") + " to configure.");
    process.exit(1);
  }

  const db = await initDatabase(config.dbPath);
  const stmts = prepareStatements(db);
  const ctx = {
    db,
    config,
    stmts,
    embed,
    insertVec: (r, e) => insertVec(stmts, r, e),
    deleteVec: (r) => deleteVec(stmts, r),
  };

  const stats = await reindex(ctx, { fullSync: true });

  db.close();
  console.log(green("✓ Reindex complete"));
  console.log(`  ${green("+")} ${stats.added} added`);
  console.log(`  ${yellow("~")} ${stats.updated} updated`);
  console.log(`  ${red("-")} ${stats.removed} removed`);
  console.log(`  ${dim("·")} ${stats.unchanged} unchanged`);
}

// ─── Status Command ──────────────────────────────────────────────────────────

async function runStatus() {
  const { resolveConfig } = await import("@context-vault/core/core/config");
  const { initDatabase } = await import("@context-vault/core/index/db");
  const { gatherVaultStatus } = await import("@context-vault/core/core/status");

  const config = resolveConfig();
  const db = await initDatabase(config.dbPath);

  const status = gatherVaultStatus({ db, config });

  db.close();

  console.log();
  console.log(`  ${bold("◇ context-vault")} ${dim(`v${VERSION}`)}`);
  console.log();
  console.log(`  Vault:     ${config.vaultDir} ${dim(`(${config.vaultDirExists ? status.fileCount + " files" : "missing"})`)}`);
  console.log(`  Database:  ${config.dbPath} ${dim(`(${status.dbSize})`)}`);
  console.log(`  Dev dir:   ${config.devDir}`);
  console.log(`  Data dir:  ${config.dataDir}`);
  console.log(`  Config:    ${config.configPath} ${dim(`(${existsSync(config.configPath) ? "exists" : "missing"})`)}`);
  console.log(`  Resolved:  ${status.resolvedFrom}`);
  console.log(`  Schema:    v5 (categories)`);

  if (status.kindCounts.length) {
    const BAR_WIDTH = 20;
    const maxCount = Math.max(...status.kindCounts.map((k) => k.c));
    console.log();
    console.log(bold("  Indexed"));
    for (const { kind, c } of status.kindCounts) {
      const filled = maxCount > 0 ? Math.round((c / maxCount) * BAR_WIDTH) : 0;
      const bar = "█".repeat(filled) + "░".repeat(BAR_WIDTH - filled);
      const countStr = String(c).padStart(4);
      console.log(`  ${countStr} ${kind}s   ${dim(bar)}`);
    }
  } else {
    console.log(`\n  ${dim("(empty — no entries indexed)")}`);
  }

  if (status.embeddingStatus) {
    const { indexed, total, missing } = status.embeddingStatus;
    if (missing > 0) {
      const BAR_WIDTH = 20;
      const filled = total > 0 ? Math.round((indexed / total) * BAR_WIDTH) : 0;
      const bar = "█".repeat(filled) + "░".repeat(BAR_WIDTH - filled);
      const pct = total > 0 ? Math.round((indexed / total) * 100) : 0;
      console.log();
      console.log(`  Embeddings ${dim(bar)} ${indexed}/${total} (${pct}%)`);
    }
  }

  if (status.subdirs.length) {
    console.log();
    console.log(bold("  Disk Directories"));
    for (const { name, count } of status.subdirs) {
      console.log(`    ${name}/: ${count} files`);
    }
  }

  if (status.stalePaths) {
    console.log();
    console.log(yellow("  Stale paths detected in DB."));
    console.log(`  Run ${cyan("context-mcp reindex")} to update.`);
  }
  console.log();
}

// ─── Update Command ─────────────────────────────────────────────────────────

async function runUpdate() {
  console.log();
  console.log(`  ${bold("◇ context-vault")} ${dim(`v${VERSION}`)}`);
  console.log();

  let latest;
  try {
    latest = execSync("npm view context-vault version", { encoding: "utf-8", stdio: ["pipe", "pipe", "pipe"] }).trim();
  } catch {
    console.error(red("  Could not check for updates. Verify your network connection."));
    return;
  }

  if (latest === VERSION) {
    console.log(green("  Already up to date."));
    console.log();
    return;
  }

  console.log(`  Current: ${dim(VERSION)}`);
  console.log(`  Latest:  ${green(latest)}`);
  console.log();

  if (!isNonInteractive) {
    const answer = await prompt(`  Update to v${latest}? (Y/n):`, "Y");
    if (answer.toLowerCase() === "n") {
      console.log(dim("  Cancelled."));
      return;
    }
  }

  console.log(dim("  Installing..."));
  try {
    execSync("npm install -g context-vault@latest", { stdio: "inherit" });
    console.log();
    console.log(green(`  ✓ Updated to v${latest}`));
  } catch {
    console.error(red("  Update failed. Try manually: npm install -g context-vault@latest"));
  }
  console.log();
}

// ─── Uninstall Command ──────────────────────────────────────────────────────

async function runUninstall() {
  console.log();
  console.log(`  ${bold("◇ context-vault")} ${dim("uninstall")}`);
  console.log();

  // Remove from Claude Code
  try {
    const env = { ...process.env };
    delete env.CLAUDECODE;
    execSync("claude mcp remove context-mcp -s user", { stdio: "pipe", env });
    console.log(`  ${green("+")} Removed from Claude Code`);
  } catch {
    console.log(`  ${dim("-")} Claude Code — not configured or not installed`);
  }

  // Remove from JSON-configured tools
  for (const tool of TOOLS.filter((t) => t.configType === "json")) {
    if (!existsSync(tool.configPath)) continue;
    try {
      const config = JSON.parse(readFileSync(tool.configPath, "utf-8"));
      if (config[tool.configKey]?.["context-mcp"]) {
        delete config[tool.configKey]["context-mcp"];
        writeFileSync(tool.configPath, JSON.stringify(config, null, 2) + "\n");
        console.log(`  ${green("+")} Removed from ${tool.name}`);
      }
    } catch {
      console.log(`  ${dim("-")} ${tool.name} — could not update config`);
    }
  }

  // Optionally remove data directory
  const dataDir = join(HOME, ".context-mcp");
  if (existsSync(dataDir)) {
    console.log();
    const answer = isNonInteractive
      ? "n"
      : await prompt(`  Remove data directory (${dataDir})? (y/N):`, "N");
    if (answer.toLowerCase() === "y") {
      const { rmSync } = await import("node:fs");
      rmSync(dataDir, { recursive: true, force: true });
      console.log(`  ${green("+")} Removed ${dataDir}`);
    } else {
      console.log(`  ${dim("Kept")} ${dataDir}`);
    }
  }

  console.log();
  console.log(dim("  Vault directory was not touched (your knowledge files are safe)."));
  console.log(`  To fully remove: ${cyan("npm uninstall -g context-vault")}`);
  console.log();
}

// ─── Migrate Command ─────────────────────────────────────────────────────────

async function runMigrate() {
  const direction = args.includes("--to-hosted") ? "to-hosted"
    : args.includes("--to-local") ? "to-local"
    : null;

  if (!direction) {
    console.log(`\n  ${bold("context-mcp migrate")}\n`);
    console.log(`  Usage:`);
    console.log(`    context-mcp migrate --to-hosted  Upload local vault to hosted service`);
    console.log(`    context-mcp migrate --to-local   Download hosted vault to local files`);
    console.log(`\n  Options:`);
    console.log(`    --url <url>      Hosted server URL (default: https://vault.contextvault.dev)`);
    console.log(`    --key <key>      API key (cv_...)`);
    console.log();
    return;
  }

  const hostedUrl = getFlag("--url") || "https://vault.contextvault.dev";
  const apiKey = getFlag("--key");

  if (!apiKey) {
    console.error(red("  Error: --key <api_key> is required for migration."));
    console.error(`  Get your API key at ${cyan(hostedUrl + "/dashboard")}`);
    return;
  }

  const { resolveConfig } = await import("@context-vault/core/core/config");
  const config = resolveConfig();

  if (direction === "to-hosted") {
    const { migrateToHosted } = await import("@context-vault/hosted/migration/migrate");
    console.log(`\n  ${bold("Migrating to hosted")}...`);
    console.log(dim(`  Vault: ${config.vaultDir}`));
    console.log(dim(`  Target: ${hostedUrl}\n`));

    const results = await migrateToHosted({
      vaultDir: config.vaultDir,
      hostedUrl,
      apiKey,
      log: (msg) => console.log(`  ${dim(msg)}`),
    });

    console.log(`\n  ${green("+")} ${results.uploaded} entries uploaded`);
    if (results.failed > 0) {
      console.log(`  ${red("-")} ${results.failed} failed`);
      for (const err of results.errors.slice(0, 5)) {
        console.log(`    ${dim(err)}`);
      }
    }
    console.log(dim("\n  Your local vault was not modified (safe backup)."));
  } else {
    const { migrateToLocal } = await import("@context-vault/hosted/migration/migrate");
    console.log(`\n  ${bold("Migrating to local")}...`);
    console.log(dim(`  Source: ${hostedUrl}`));
    console.log(dim(`  Target: ${config.vaultDir}\n`));

    const results = await migrateToLocal({
      vaultDir: config.vaultDir,
      hostedUrl,
      apiKey,
      log: (msg) => console.log(`  ${dim(msg)}`),
    });

    console.log(`\n  ${green("+")} ${results.downloaded} entries restored`);
    if (results.failed > 0) {
      console.log(`  ${red("-")} ${results.failed} failed`);
    }
    console.log(dim("\n  Run `context-mcp reindex` to rebuild the search index."));
  }
  console.log();
}

// ─── Serve Command ──────────────────────────────────────────────────────────

async function runServe() {
  await import("../src/server/index.js");
}

// ─── Main Router ─────────────────────────────────────────────────────────────

async function main() {
  if (flags.has("--version") || command === "version") {
    console.log(VERSION);
    return;
  }

  if (flags.has("--help") || command === "help" || !command) {
    showHelp();
    return;
  }

  switch (command) {
    case "setup":
      await runSetup();
      break;
    case "serve":
      await runServe();
      break;
    case "ui":
      runUi();
      break;
    case "import":
    case "export":
      console.log(`Import/export removed. Add .md files to vault/ and run \`context-mcp reindex\`.`);
      break;
    case "reindex":
      await runReindex();
      break;
    case "status":
      await runStatus();
      break;
    case "update":
      await runUpdate();
      break;
    case "uninstall":
      await runUninstall();
      break;
    case "migrate":
      await runMigrate();
      break;
    default:
      console.error(red(`Unknown command: ${command}`));
      console.error(`Run ${cyan("context-mcp --help")} for usage.`);
      process.exit(1);
  }
}

main().catch((e) => {
  console.error(red(e.message));
  process.exit(1);
});
