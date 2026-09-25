import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import type {
  VaultBackend,
  SaveOptions,
  SaveResult,
  SearchOptions,
  SearchResultEntry,
  HealthResult,
} from './types.js';

export interface LocalOptions {
  /** Directory for the vault data dir (sets CONTEXT_VAULT_DATA_DIR for the child). */
  dir?: string;
  /** Path to the context-vault binary. Defaults to $CONTEXT_VAULT_BIN or `context-vault`. */
  bin?: string;
}

/**
 * Logic-free local backend: spawns the Rust `context-vault mcp` binary and drives it
 * over MCP JSON-RPC on stdio. Contains NO vault business logic and does NOT depend on
 * `@context-vault/core` — the Rust engine is the single implementation.
 */
export class LocalBackend implements VaultBackend {
  private readonly bin: string;
  private readonly env: NodeJS.ProcessEnv;
  private proc: ChildProcessWithoutNullStreams | null = null;
  private buf: Buffer = Buffer.alloc(0);
  private nextId = 1;
  private pending = new Map<number, { resolve: (v: any) => void; reject: (e: Error) => void }>();
  private ready: Promise<void> | null = null;

  constructor(opts?: LocalOptions) {
    this.bin = opts?.bin || process.env.CONTEXT_VAULT_BIN || 'context-vault';
    this.env = { ...process.env };
    if (opts?.dir) this.env.CONTEXT_VAULT_DATA_DIR = opts.dir;
  }

  private ensureStarted(): Promise<void> {
    if (this.ready) return this.ready;
    this.ready = new Promise((resolve, reject) => {
      const proc = spawn(this.bin, ['mcp'], { env: this.env });
      this.proc = proc;
      proc.on('error', (e) =>
        reject(new Error(`Failed to spawn "${this.bin} mcp": ${e.message}. Install context-vault or set CONTEXT_VAULT_BIN.`))
      );
      proc.stdout.on('data', (chunk: Buffer) => this.onData(chunk));
      // Initialize handshake, then resolve.
      this.request('initialize', {}).then(() => resolve()).catch(reject);
    });
    return this.ready;
  }

  /**
   * Parse Content-Length framed JSON-RPC messages from the child's stdout.
   *
   * Operates on raw bytes (Buffer), NOT on a decoded JS string. Content-Length
   * counts UTF-8 bytes, but a JS string's `.length` counts UTF-16 code units, so
   * any multibyte character in a message (e.g. the "✓" in the engine's
   * `save_context` reply) makes a string-based length check come up short of the
   * declared byte length — the frame never completes and the caller hangs forever.
   * Slicing on byte offsets and decoding only the extracted body avoids that.
   */
  private onData(chunk: Buffer): void {
    this.buf = this.buf.length ? Buffer.concat([this.buf, chunk]) : chunk;
    const sep = Buffer.from('\r\n\r\n');
    while (true) {
      const headerEnd = this.buf.indexOf(sep);
      if (headerEnd === -1) return;
      const header = this.buf.subarray(0, headerEnd).toString('utf-8');
      const match = header.match(/Content-Length:\s*(\d+)/i);
      if (!match) {
        this.buf = this.buf.subarray(headerEnd + 4);
        continue;
      }
      const len = parseInt(match[1], 10);
      const start = headerEnd + 4;
      if (this.buf.length < start + len) return;
      const body = this.buf.subarray(start, start + len).toString('utf-8');
      this.buf = this.buf.subarray(start + len);
      this.dispatch(body);
    }
  }

  private dispatch(body: string): void {
    let msg: any;
    try {
      msg = JSON.parse(body);
    } catch {
      return;
    }
    const waiter = this.pending.get(msg.id);
    if (!waiter) return;
    this.pending.delete(msg.id);
    if (msg.error) waiter.reject(new Error(msg.error.message || 'MCP error'));
    else waiter.resolve(msg.result);
  }

  private request(method: string, params: unknown): Promise<any> {
    const id = this.nextId++;
    const payload = JSON.stringify({ jsonrpc: '2.0', id, method, params });
    const frame = `Content-Length: ${Buffer.byteLength(payload, 'utf-8')}\r\n\r\n${payload}`;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.proc!.stdin.write(frame);
    });
  }

  private async callTool(name: string, args: Record<string, unknown>): Promise<string> {
    await this.ensureStarted();
    const result = await this.request('tools/call', { name, arguments: args });
    const content = result?.content;
    if (Array.isArray(content)) {
      return content.map((c: any) => (typeof c?.text === 'string' ? c.text : '')).join('\n');
    }
    return '';
  }

  async save(options: SaveOptions): Promise<SaveResult> {
    const text = await this.callTool('save_context', {
      kind: options.kind,
      title: options.title,
      body: options.body,
      tags: options.tags,
      tier: options.tier,
      identity_key: options.identityKey,
      source: options.source,
      meta: options.meta,
    });
    const id = text.match(/\b([0-9A-HJKMNP-TV-Z]{26})\b/)?.[1] || text.trim();
    // A just-saved entry is maximally fresh; the engine owns decay scoring thereafter.
    return { id, freshness: { score: 100, label: 'fresh' } };
  }

  async search(query: string, options?: SearchOptions): Promise<SearchResultEntry[]> {
    const text = await this.callTool('get_context', {
      query,
      limit: options?.limit ?? 20,
      kind: options?.kind,
      tags: options?.tags,
    });
    // Rust MCP's get_context returns a markdown list, one result per line:
    // "- **title** (score: 0.92) [kind/tier] id: `01H...`", optionally followed by
    // an indented first-line-of-body line. PARITY: a structured JSON tool result
    // would be more robust than parsing this format; not built yet.
    const resultLine = /^-\s+\*\*(.+?)\*\*\s+\(score:\s*([\d.]+)\)\s+\[([^\/\]]+)\/([^\]]+)\]\s+id:\s*`([^`]+)`/;
    const entries: SearchResultEntry[] = [];
    for (const line of text.split('\n')) {
      const match = line.match(resultLine);
      if (match) {
        const [, title, score, kind, tier, id] = match;
        entries.push({
          id,
          kind,
          title,
          body: '',
          tags: [],
          score: parseFloat(score),
          freshness: { score: 0, label: tier },
        });
      } else if (line.startsWith('  ') && entries.length > 0) {
        entries[entries.length - 1].body = line.trim();
      }
    }
    return entries;
  }

  async health(): Promise<HealthResult> {
    const text = await this.callTool('context_status', {});
    const total = parseInt(text.match(/\*\*Total entries\*\*\s*\|\s*(\d+)/)?.[1] || '0', 10);
    const byKind: Record<string, { total: number; avgScore: number }> = {};
    for (const match of text.matchAll(/\|\s*·\s*(\S+)\s*\|\s*(\d+)\s*\|/g)) {
      byKind[match[1]] = { total: parseInt(match[2], 10), avgScore: 0 };
    }
    // PARITY: distribution/averageScore/needsAttention have no backing Rust engine
    // query yet (context_status doesn't compute freshness tiers) — not fabricated.
    return {
      total,
      distribution: { fresh: 0, aging: 0, stale: 0, dormant: 0 },
      averageScore: 0,
      needsAttention: 0,
      byKind,
    };
  }

  async delete(id: string): Promise<void> {
    await this.callTool('delete_context', { id });
  }

  /**
   * Shut down the spawned engine process, if one was started. Closing stdin gives
   * the engine an EOF so its read loop exits cleanly; `kill()` is the backstop.
   * Safe to call more than once, and a no-op if the backend never started.
   */
  async close(): Promise<void> {
    const proc = this.proc;
    if (!proc) return;
    this.proc = null;
    this.ready = null;
    for (const waiter of this.pending.values()) {
      waiter.reject(new Error('LocalBackend closed before request completed'));
    }
    this.pending.clear();
    proc.stdin.end();
    proc.kill();
  }
}
