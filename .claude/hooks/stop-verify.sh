#!/usr/bin/env bash
# Stop hook: surface uncommitted changes before Claude finishes
# Always exits 0 (informational only — does not force Claude to continue)

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

STATUS=$(git status --short)

if [[ -n "$STATUS" ]]; then
  echo "Uncommitted changes detected:"
  echo "$STATUS"
fi

exit 0
