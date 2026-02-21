---
sidebar_position: 1
title: Installation
---

# Installation

## Linux / macOS (recommended)

```bash
curl -fsSL \
  https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.sh \
  | sh
```

The script auto-detects your OS and architecture, verifies the SHA-256 checksum, and installs the binary to `/usr/local/bin/merlin` (or `~/.local/bin/merlin` if you don't have root).

### Options

| Variable | Default | Description |
|---|---|---|
| `MERLIN_VERSION` | `latest` | Pin to a specific release, e.g. `v1.2.0` |
| `MERLIN_INSTALL_DIR` | `/usr/local/bin` | Override install directory |
| `MERLIN_MUSL` | `1` (Linux) | Use the musl static binary (best for CI) |
| `MERLIN_NO_VERIFY` | unset | Skip SHA-256 verification |

```bash
# Install a specific version to a custom directory
MERLIN_VERSION=v1.0.0 MERLIN_INSTALL_DIR=~/.local/bin \
  curl -fsSL .../install.sh | sh
```

## Windows (PowerShell)

```powershell
irm https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.ps1 | iex
```

The script downloads `merlin-windows-amd64.exe`, verifies SHA-256, installs it to `~\.merlin\bin`, and adds that path to your user `PATH`.

## Docker

```bash
docker pull ghcr.io/arunachalamkalimuthu/merlin:latest

docker run --rm \
  -e GITHUB_TOKEN=... \
  -e ANTHROPIC_API_KEY=... \
  -e GITHUB_ACTIONS=true \
  -e GITHUB_REPOSITORY=owner/repo \
  -e GITHUB_SHA=abc123 \
  ghcr.io/arunachalamkalimuthu/merlin:latest review
```

## Build from source

Requires Rust 1.75+.

```bash
git clone https://github.com/Arunachalamkalimuthu/merlin-ai-code-review.git
cd merlin-ai-code-review
cargo build --release
# Binary at: ./target/release/merlin
```

## Verify installation

```bash
merlin --version
merlin --help
```
