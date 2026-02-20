#!/usr/bin/env bash
# PostToolUse hook: run npm test when a source file in core/local/hosted is edited
# Skips test files (handled by global hook) and non-source paths

set -euo pipefail

INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

if [[ -z "$FILE_PATH" ]]; then
  exit 0
fi

# Only fire on source files in core/local/hosted packages
if ! echo "$FILE_PATH" | grep -qE 'packages/(core|local|hosted)/src/.+\.js$'; then
  exit 0
fi

# Skip test and spec files (global hook handles those)
if echo "$FILE_PATH" | grep -qE '\.(test|spec)\.js$'; then
  exit 0
fi

echo "Running tests for edited source file: $FILE_PATH"
cd "$(git rev-parse --show-toplevel)"
npm test 2>&1 | tail -20

exit 0
