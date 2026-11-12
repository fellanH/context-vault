#!/usr/bin/env bash
# Install the native context-vault binary — no Node, no npm.
#
#   curl -fsSL https://raw.githubusercontent.com/fellanH/context-vault/main/install.sh | sh
#
# Env overrides:
#   CONTEXT_VAULT_VERSION      release tag to install, e.g. v4.0.0 (default: latest)
#   CONTEXT_VAULT_INSTALL_DIR  install directory (default: $HOME/.local/bin)
set -euo pipefail

REPO="fellanH/context-vault"
INSTALL_DIR="${CONTEXT_VAULT_INSTALL_DIR:-$HOME/.local/bin}"

info() { printf '%s\n' "$*" >&2; }
die() {
  printf 'context-vault: %s\n' "$*" >&2
  exit 1
}

detect_target() {
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os" in
    Darwin)
      case "$arch" in
        arm64) echo "aarch64-apple-darwin" ;;
        x86_64) echo "x86_64-apple-darwin" ;;
        *) die "unsupported macOS architecture: $arch" ;;
      esac
      ;;
    Linux)
      case "$arch" in
        aarch64 | arm64) echo "aarch64-unknown-linux-gnu" ;;
        x86_64) echo "x86_64-unknown-linux-gnu" ;;
        *) die "unsupported Linux architecture: $arch" ;;
      esac
      ;;
    MINGW* | MSYS* | CYGWIN*)
      die "use install.ps1 on Windows: https://raw.githubusercontent.com/$REPO/main/install.ps1"
      ;;
    *) die "unsupported OS: $os" ;;
  esac
}

resolve_version() {
  if [ -n "${CONTEXT_VAULT_VERSION:-}" ]; then
    printf '%s' "$CONTEXT_VAULT_VERSION"
    return
  fi
  curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | grep '"tag_name"' \
    | head -n1 \
    | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
}

verify_checksum() {
  archive="$1"
  sums="$2"
  if command -v sha256sum >/dev/null 2>&1; then
    computed="$(sha256sum "$archive" | awk '{print $1}')"
  elif command -v shasum >/dev/null 2>&1; then
    computed="$(shasum -a 256 "$archive" | awk '{print $1}')"
  else
    info "warning: no sha256sum/shasum available, skipping checksum verification"
    return
  fi
  expected="$(grep " $(basename "$archive")\$" "$sums" | awk '{print $1}')"
  [ -n "$expected" ] || die "no checksum entry found for $(basename "$archive")"
  [ "$expected" = "$computed" ] || die "checksum mismatch for $(basename "$archive") (expected $expected, got $computed)"
}

main() {
  command -v curl >/dev/null 2>&1 || die "curl is required"
  command -v tar >/dev/null 2>&1 || die "tar is required"

  target="$(detect_target)"
  version="$(resolve_version)"
  [ -n "$version" ] || die "could not resolve latest release version"

  archive_name="context-vault-${target}.tar.gz"
  base_url="https://github.com/$REPO/releases/download/$version"

  workdir="$(mktemp -d)"
  trap 'rm -rf "$workdir"' EXIT

  info "Downloading context-vault $version ($target)..."
  curl -fsSL -o "$workdir/$archive_name" "$base_url/$archive_name" \
    || die "download failed: $base_url/$archive_name (release may not exist yet for this version)"
  curl -fsSL -o "$workdir/SHA256SUMS" "$base_url/SHA256SUMS" \
    || die "download failed: $base_url/SHA256SUMS"

  verify_checksum "$workdir/$archive_name" "$workdir/SHA256SUMS"

  tar xzf "$workdir/$archive_name" -C "$workdir"
  mkdir -p "$INSTALL_DIR"
  mv "$workdir/context-vault" "$INSTALL_DIR/context-vault"
  chmod +x "$INSTALL_DIR/context-vault"

  info "Installed context-vault $version to $INSTALL_DIR/context-vault"

  cmd="context-vault"
  case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
      cmd="$INSTALL_DIR/context-vault"
      case "${SHELL:-}" in
        */zsh) profile="~/.zshrc" ;;
        */bash) profile="~/.bashrc" ;;
        *) profile="your shell profile" ;;
      esac
      info ""
      info "$INSTALL_DIR is not on your PATH. Add it to $profile, then open a new terminal:"
      info "  echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> $profile"
      ;;
  esac

  info ""
  info "Next:"
  info "  $cmd setup        # connects Claude Code, Cursor and the other AI tools it finds"
  info "Then restart your AI tool and say: \"Save to my vault: ...\". In a new session: \"Search my vault: ...\""
}

main "$@"
