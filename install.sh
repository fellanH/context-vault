#!/bin/sh
set -e

# context-mcp installer
# Usage: curl -fsSL https://raw.githubusercontent.com/fellanH/context-mcp/main/install.sh | sh

PACKAGE="@fellanh/context-mcp"
MIN_NODE=20

echo "context-mcp installer"
echo "====================="
echo ""

# Check for Node.js
if ! command -v node >/dev/null 2>&1; then
  echo "Error: Node.js is not installed."
  echo "Install Node.js $MIN_NODE+ from https://nodejs.org"
  exit 1
fi

# Check Node.js version
NODE_VERSION=$(node -p "process.versions.node.split('.')[0]")
if [ "$NODE_VERSION" -lt "$MIN_NODE" ]; then
  echo "Error: Node.js $MIN_NODE+ required (found v$(node -v))"
  echo "Update from https://nodejs.org"
  exit 1
fi

echo "Node.js $(node -v) detected"
echo ""

# Install globally
echo "Installing $PACKAGE..."
npm install -g "$PACKAGE"
echo ""

# Run interactive setup
echo "Running setup..."
context-mcp setup
