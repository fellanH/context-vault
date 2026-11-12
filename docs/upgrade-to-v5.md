# Upgrading a vault to v5

v5 is an opt-in engine in this release. Run one command and your vault moves onto it. If you don't, nothing changes: v4 keeps working exactly as before.

## What changes

v4 kept everything in one place: your notes, plus every session log, tool event and chat transcript your agents wrote. On a busy machine those machine records outnumber your notes by more than ten to one, and they crowd search.

v5 splits them:

- **Your notes** stay in your vault folder, one markdown file each, as today. Search covers them by default.
- **Machine records** go into a separate *data vault* next to it (`<your vault>-exhaust`), stored as compressed daily files. You can still search them by name, and read them without the tool (`zstd -dc <file>.jsonl.zst | grep <term>`).

v5 also drops SQLite and the embedding model. Search is full-text (BM25) over plain files. Nothing runs at startup, and a full rebuild of a few thousand notes takes seconds.

## Before you start

1. Make sure your vault folder is backed up. A git repository is ideal: migrate writes one line into some notes (below), and `git diff` shows exactly which.
2. Check the free disk space. The data vault is compressed, but it starts out as a copy of your machine records.
3. Plan to restart your AI clients afterwards: a client that is already running keeps using v4 until it restarts.

## Run it

```
context-vault migrate
```

It prints where it copies from and to, then works through these steps:

1. Every note without an `id` in its frontmatter gets the id v4 already knew it by, so links and agents keep working. This is the only change to your notes. The report lists the files.
2. Machine records are copied into the data vault. Chat transcripts under `events/chat-logs` are included unless you pass `--no-chat-logs`.
3. The copy is verified: counts per kind, every id present exactly once, and sampled records compared byte for byte with their source.
4. Both search indexes are built.
5. Only then is `~/.context-mcp/vaults.toml` written. That file is what switches this machine to v5.

If any step fails, `vaults.toml` is not written and v4 stays in use. Fix what the error names (for example two notes sharing one id) and run the command again; it starts over cleanly.

Large vaults take a few minutes. Options: `--exhaust <dir>` puts the data vault somewhere else (it must be empty), and `--no-chat-logs` leaves transcripts out.

## After

- Restart your AI clients. They pick up v5 on start; the tools have the same names.
- `context-vault status` shows the engine and your vaults. `context-vault vault list` shows each vault, its type, and whether default search covers it.
- `context-vault reindex` picks up files you edited outside the tool. `reindex --full --plan` shows what a full rebuild would read and how long it would take before it starts.

## What is not removed

The original machine-record files stay in your vault folder, and the v4 database stays on disk. v5 ignores both. Removing them is a separate, later step, and nothing in this release does it for you.

## Going back

Delete `~/.context-mcp/vaults.toml`, run `context-vault reindex` (so v4 picks up notes saved while on v5) and restart your AI clients: v4 runs again on its database. The data vault and the v5 indexes can then be deleted. The ids written into your notes are ordinary frontmatter, and v4 reads them too.
