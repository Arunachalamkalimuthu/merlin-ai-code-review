#!/usr/bin/env sh
# Merlin installer — Linux & macOS
# Usage:
#   curl -fsSL https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.sh | sh
#
# Environment variables:
#   MERLIN_VERSION      Pin a release tag, e.g. "v1.2.0" (default: latest)
#   MERLIN_INSTALL_DIR  Where to put the binary (default: /usr/local/bin or ~/.local/bin)
#   MERLIN_MUSL         Set to "1" to force the musl (static) build — recommended for Alpine CI
#   MERLIN_NO_VERIFY    Set to "1" to skip SHA-256 verification (not recommended)

set -eu

REPO="Arunachalamkalimuthu/merlin-ai-code-review"
BINARY="merlin"

# ── Helpers ────────────────────────────────────────────────────────────────────

say()  { printf '\033[1;32m[merlin]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[merlin]\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31m[merlin] error:\033[0m %s\n' "$*" >&2; exit 1; }

need() {
    command -v "$1" >/dev/null 2>&1 || die "Required command not found: $1"
}

# ── Detect OS ──────────────────────────────────────────────────────────────────

OS="$(uname -s)"
case "$OS" in
    Linux)  OS_NAME="linux"  ;;
    Darwin) OS_NAME="darwin" ;;
    *)      die "Unsupported operating system: $OS. Use install.ps1 on Windows." ;;
esac

# ── Detect architecture ────────────────────────────────────────────────────────

ARCH="$(uname -m)"
case "$ARCH" in
    x86_64|amd64)   ARCH_NAME="amd64" ;;
    aarch64|arm64)  ARCH_NAME="arm64" ;;
    *)              die "Unsupported architecture: $ARCH" ;;
esac

# ── Build asset name ───────────────────────────────────────────────────────────

MUSL="${MERLIN_MUSL:-0}"

if [ "$OS_NAME" = "linux" ] && [ "$MUSL" = "1" ]; then
    ASSET="${BINARY}-linux-${ARCH_NAME}-musl"
elif [ "$OS_NAME" = "linux" ]; then
    # Default to musl on Linux for broadest compatibility (works on Alpine too)
    ASSET="${BINARY}-linux-${ARCH_NAME}-musl"
else
    ASSET="${BINARY}-${OS_NAME}-${ARCH_NAME}"
fi

# ── Resolve version ────────────────────────────────────────────────────────────

VERSION="${MERLIN_VERSION:-}"
if [ -z "$VERSION" ] || [ "$VERSION" = "latest" ]; then
    need curl
    say "Fetching latest release tag..."
    VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
              | grep '"tag_name"' \
              | sed -E 's/.*"([^"]+)".*/\1/')
    [ -n "$VERSION" ] || die "Could not determine latest release version. Check your internet connection."
fi

say "Installing Merlin ${VERSION} (${ASSET})..."

BASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"

# ── Download ───────────────────────────────────────────────────────────────────

TMPDIR=$(mktemp -d)
# shellcheck disable=SC2064
trap "rm -rf '$TMPDIR'" EXIT

download() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL --progress-bar "$1" -o "$2"
    elif command -v wget >/dev/null 2>&1; then
        wget -q --show-progress "$1" -O "$2"
    else
        die "Neither curl nor wget found. Install one and retry."
    fi
}

ASSET_PATH="${TMPDIR}/${ASSET}"
ASSET_URL="${BASE_URL}/${ASSET}"

say "Downloading ${ASSET_URL}"
download "${ASSET_URL}" "${ASSET_PATH}"

# ── Verify checksum ────────────────────────────────────────────────────────────

NO_VERIFY="${MERLIN_NO_VERIFY:-0}"
if [ "$NO_VERIFY" != "1" ]; then
    SHA256_URL="${BASE_URL}/${ASSET}.sha256"
    SHA256_PATH="${TMPDIR}/${ASSET}.sha256"

    say "Downloading checksum..."
    if download "${SHA256_URL}" "${SHA256_PATH}" 2>/dev/null; then
        say "Verifying checksum..."
        EXPECTED=$(awk '{print $1}' "${SHA256_PATH}")

        if command -v sha256sum >/dev/null 2>&1; then
            ACTUAL=$(sha256sum "${ASSET_PATH}" | awk '{print $1}')
        elif command -v shasum >/dev/null 2>&1; then
            ACTUAL=$(shasum -a 256 "${ASSET_PATH}" | awk '{print $1}')
        else
            warn "sha256sum / shasum not found — skipping checksum verification."
            ACTUAL="$EXPECTED"
        fi

        if [ "$ACTUAL" != "$EXPECTED" ]; then
            die "Checksum mismatch!\n  Expected: ${EXPECTED}\n  Got:      ${ACTUAL}\n\nThe download may be corrupt. Try again."
        fi
        say "Checksum OK."
    else
        warn "Could not download checksum file — skipping verification."
    fi
fi

# ── Install ────────────────────────────────────────────────────────────────────

chmod +x "${ASSET_PATH}"

# Determine install directory
if [ -n "${MERLIN_INSTALL_DIR:-}" ]; then
    INSTALL_DIR="$MERLIN_INSTALL_DIR"
elif [ -w "/usr/local/bin" ]; then
    INSTALL_DIR="/usr/local/bin"
elif [ -w "${HOME}/.local/bin" ]; then
    INSTALL_DIR="${HOME}/.local/bin"
else
    INSTALL_DIR="${HOME}/.local/bin"
    mkdir -p "$INSTALL_DIR"
fi

DEST="${INSTALL_DIR}/${BINARY}"

if [ -w "$INSTALL_DIR" ]; then
    mv "${ASSET_PATH}" "${DEST}"
else
    say "Need sudo to install to ${INSTALL_DIR}..."
    sudo mv "${ASSET_PATH}" "${DEST}"
fi

say "Installed → ${DEST}"

# ── PATH hint ─────────────────────────────────────────────────────────────────

case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) ;;  # already in PATH
    *)
        warn "${INSTALL_DIR} is not in your PATH."
        warn "Add this to your shell profile:"
        warn "  export PATH=\"${INSTALL_DIR}:\$PATH\""
        ;;
esac

# ── Done ───────────────────────────────────────────────────────────────────────

say "Done! Run: merlin --help"
