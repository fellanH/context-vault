#!/usr/bin/env bash
# check-stay-split.sh — done-bar STAY-SPLIT / SIZE ratchet gate (tas-25c61576).
#
# context-vault is the ZERO-GRANDFATHER PILOT: no grandfathered files —
# every Rust file must fit within the contract ceiling (300 LOC) from day one.
#
# Gates (via tidy ratchet):
#   • File size ceiling: new .rs files must be ≤ 300 LOC.
#   • Crate ceiling: no crate may exceed 4000 LOC / 15 files.
#   • Drift can only shrink (enforced by architecture.json + contracts.toml).
#
# Exit 0 = GREEN (all units at/below their ratchet ceiling).
# Exit 1 = RED   (one or more units exceed the ratchet) → done-bar fails.
# Exit 2 = ERROR (tidy binary with ratchet support not found).
#
# Usage:
#   scripts/check-stay-split.sh              # ratchet this repo
#   scripts/check-stay-split.sh --json       # machine-readable report
#   scripts/check-stay-split.sh --summary    # terse pass/fail only
#
# Escape hatch (audited, never default): ALLOW_STRUCTURE_DRIFT=1 bypasses the
# gate — you MUST pair it with a filed split row (same contract as pre-commit).
set -euo pipefail

if git rev-parse --show-toplevel &>/dev/null; then
  REPO_ROOT="$(git rev-parse --show-toplevel)"
else
  REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
fi

FORMAT="human"
SUMMARY_ONLY=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --json)    FORMAT="json"; shift ;;
    --summary) SUMMARY_ONLY=1; shift ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

if [[ "${ALLOW_STRUCTURE_DRIFT:-}" == "1" ]]; then
  echo "check-stay-split: stay-split ratchet BYPASSED (ALLOW_STRUCTURE_DRIFT=1) — file a split row." >&2
  exit 0
fi

# Locate a tidy binary that supports `tidy ratchet`.
supports_ratchet() { "$1" ratchet --help >/dev/null 2>&1; }

TIDY_BIN=""
SIBLING_TIDY="$(cd "$REPO_ROOT/.." && pwd)/tidy"

for cand in \
  "tidy" \
  "$SIBLING_TIDY/target/release/tidy" \
  "$SIBLING_TIDY/target/debug/tidy"; do
  if command -v "$cand" >/dev/null 2>&1 && supports_ratchet "$cand"; then
    TIDY_BIN="$cand"
    break
  fi
done

if [[ -z "$TIDY_BIN" ]]; then
  echo "check-stay-split: no tidy binary with 'ratchet' support found." >&2
  echo "  Build it: cargo build --release  (in omni-os/tidy)" >&2
  exit 2
fi

if [[ "$SUMMARY_ONLY" -eq 1 ]]; then
  result="$("$TIDY_BIN" --root "$REPO_ROOT" --format json ratchet 2>/dev/null || true)"
  reds=$(echo "$result" | python3 -c "import sys,json; r=json.load(sys.stdin); print(len(r.get('violations', [])))" 2>/dev/null || echo "?")
  echo "stay-split [context-vault zero-grandfather]: ${reds} violations"
  "$TIDY_BIN" --root "$REPO_ROOT" --format "$FORMAT" ratchet 2>/dev/null
else
  exec "$TIDY_BIN" --root "$REPO_ROOT" --format "$FORMAT" ratchet
fi
