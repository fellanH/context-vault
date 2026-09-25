#!/usr/bin/env bash
set -euo pipefail
# Copy a real vault.db slice and run the Rust bench with dated JSON archive.
# Usage: CV_BENCH_VAULT_SRC=~/.context-mcp/vault.db ./scripts/bench-vault-dry-run.sh
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="${CV_BENCH_VAULT_SRC:-$HOME/.context-mcp/vault.db}"
ARCHIVE_DIR="${CV_BENCH_ARCHIVE_DIR:-$ROOT/artifacts/bench-runs}"
STAMP="$(date -u +%Y-%m-%dT%H-%M-%SZ)"
TMP="$(mktemp -d)"
cleanup() { rm -rf "$TMP"; }
trap cleanup EXIT
cp "$SRC" "$TMP/vault.db"
OUT="$ARCHIVE_DIR/bench-omni-vault-copy-$STAMP.json"
mkdir -p "$ARCHIVE_DIR"
cd "$ROOT"
cargo run -q -p context-vault -- bench \
  --format json \
  --disk-mode b \
  --vault-db "$TMP/vault.db" \
  --archive-json "$OUT"
echo "Archived: $OUT"
