#!/usr/bin/env bash
# scripts/install-hooks.sh — configure git to use the project's hooks and
# optionally install the required CLI tools.
#
# Usage:
#   bash scripts/install-hooks.sh            # configure hooks only
#   bash scripts/install-hooks.sh --tools    # configure hooks + install tools

set -euo pipefail

RED='\033[0;31m'
GRN='\033[0;32m'
YEL='\033[1;33m'
BLU='\033[0;34m'
RST='\033[0m'

INSTALL_TOOLS=false
for arg in "$@"; do
    case "$arg" in
        --tools) INSTALL_TOOLS=true ;;
        *) echo -e "${RED}Unknown argument: $arg${RST}"; exit 1 ;;
    esac
done

ROOT="$(git rev-parse --show-toplevel)"

echo ""
echo -e "${BLU}Merlin hook installer${RST}"
echo ""

# ── 1. Configure git hooksPath ────────────────────────────────────────────────
echo -e "→ Configuring git to use .githooks/ ..."
git config core.hooksPath .githooks
chmod +x "$ROOT/.githooks/pre-commit"
chmod +x "$ROOT/.githooks/pre-push"
echo -e "${GRN}✔ git hooks configured (.githooks/pre-commit, .githooks/pre-push)${RST}"
echo ""

# ── 2. Optionally install required tools ──────────────────────────────────────
if [ "$INSTALL_TOOLS" = true ]; then
    echo -e "→ Installing required tools..."
    echo ""

    install_cargo_tool() {
        local tool="$1"
        local crate="${2:-$1}"
        if command -v "$tool" &>/dev/null; then
            echo -e "${GRN}✔ ${tool} already installed${RST}"
        else
            echo -e "  Installing ${crate} ..."
            cargo install "$crate"
            echo -e "${GRN}✔ ${tool} installed${RST}"
        fi
    }

    install_cargo_tool typos     typos-cli
    install_cargo_tool cargo-audit cargo-audit
    install_cargo_tool cargo-deny  cargo-deny

    echo ""
fi

# ── 3. Summary ────────────────────────────────────────────────────────────────
echo -e "${GRN}Done. Hooks are active for this repository.${RST}"
echo ""
echo -e "  ${BLU}pre-commit${RST}  →  cargo fmt · cargo clippy · typos"
echo -e "  ${BLU}pre-push${RST}    →  cargo audit · cargo deny"
echo ""
echo -e "To install the required tools as well, run:"
echo -e "  ${YEL}bash scripts/install-hooks.sh --tools${RST}"
echo ""
echo -e "To skip hooks for a single operation:"
echo -e "  ${YEL}git commit --no-verify${RST}"
echo -e "  ${YEL}git push   --no-verify${RST}"
echo ""
