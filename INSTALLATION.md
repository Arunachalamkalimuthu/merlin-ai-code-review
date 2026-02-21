# Installing Merlin

This guide covers every supported installation method across macOS, Linux, Windows, and Docker.
Choose the method that best fits your environment.

---

## Table of Contents

- [Requirements](#requirements)
- [macOS](#macos)
- [Linux](#linux)
- [Windows](#windows)
- [Docker](#docker)
- [Build from Source](#build-from-source)
- [CI/CD Environments](#cicd-environments)
- [Verify Your Installation](#verify-your-installation)
- [First Review](#first-review)
- [Uninstall](#uninstall)
- [Troubleshooting](#troubleshooting)

---

## Requirements

Merlin is a single static binary — it has **no runtime dependencies**.

| Requirement | Details |
|---|---|
| Operating system | macOS 12+, Linux (glibc or musl), Windows 10/11 |
| Architecture | x86-64 (Intel/AMD) or ARM64 (Apple Silicon, AWS Graviton) |
| AI provider key | At least one: `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, etc. |
| VCS token | `GITHUB_TOKEN`, `GITLAB_TOKEN`, etc. (for posting comments) |

> **No Rust installation is needed** unless you are building from source.

---

## macOS

### Option 1 — One-liner installer (recommended)

Open Terminal and run:

```bash
curl -fsSL \
  https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.sh \
  | sh
```

The script automatically:
- Detects your CPU (Intel → `amd64`, Apple Silicon → `arm64`)
- Downloads the correct binary
- Verifies the SHA-256 checksum
- Installs to `/usr/local/bin/merlin` (or `~/.local/bin` if `/usr/local/bin` is not writable)

**Custom install directory:**

```bash
MERLIN_INSTALL_DIR="$HOME/.local/bin" \
  curl -fsSL \
    https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.sh \
  | sh
```

**Pin a specific version:**

```bash
MERLIN_VERSION="v0.2.0" \
  curl -fsSL \
    https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.sh \
  | sh
```

---

### Option 2 — Download binary manually

1. Go to the [Releases page](https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest)
2. Download the correct file:

| Mac type | Binary to download |
|---|---|
| Apple Silicon (M1 / M2 / M3 / M4) | `merlin-darwin-arm64` |
| Intel | `merlin-darwin-amd64` |

3. Install it:

```bash
# Apple Silicon
chmod +x merlin-darwin-arm64
sudo mv merlin-darwin-arm64 /usr/local/bin/merlin

# Intel
chmod +x merlin-darwin-amd64
sudo mv merlin-darwin-amd64 /usr/local/bin/merlin
```

**Verify the download (recommended):**

```bash
# Download checksum alongside the binary
curl -fsSL \
  https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/merlin-darwin-arm64.sha256 \
  -o merlin-darwin-arm64.sha256

# Check it matches
shasum -a 256 -c merlin-darwin-arm64.sha256
```

---

### Option 3 — Build from source (macOS)

See [Build from Source](#build-from-source) below.

---

## Linux

### Option 1 — One-liner installer (recommended)

```bash
curl -fsSL \
  https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.sh \
  | sh
```

On Linux the installer defaults to the **musl** (fully static) build so the binary runs on any
distribution — Ubuntu, Debian, Alpine, RHEL, Arch, etc. — without glibc version concerns.

**Force glibc build** (if you specifically need it):

```bash
MERLIN_MUSL=0 curl -fsSL \
  https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.sh \
  | sh
```

---

### Option 2 — Download binary manually

| Architecture | Binary |
|---|---|
| x86-64 (most servers/desktops) | `merlin-linux-amd64-musl` |
| ARM64 (Graviton, Raspberry Pi 4) | `merlin-linux-arm64-musl` |

```bash
# Example — x86-64
curl -fsSL \
  https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/merlin-linux-amd64-musl \
  -o merlin
chmod +x merlin
sudo mv merlin /usr/local/bin/
```

---

### Option 3 — Alpine Linux / Docker-based CI

The musl build runs directly on Alpine with no extra packages:

```sh
wget -qO /usr/local/bin/merlin \
  https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/merlin-linux-amd64-musl
chmod +x /usr/local/bin/merlin
```

---

## Windows

### Option 1 — PowerShell one-liner (recommended)

Open **PowerShell** (any version) and run:

```powershell
irm https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.ps1 | iex
```

The script downloads `merlin-windows-amd64.exe`, verifies its checksum, installs it to
`%USERPROFILE%\.merlin\bin\merlin.exe`, and prints a PATH reminder.

**Custom install directory:**

```powershell
$env:MERLIN_INSTALL_DIR = "C:\tools\merlin"
irm https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.ps1 | iex
```

**Add to PATH permanently** (run once in an elevated PowerShell):

```powershell
[System.Environment]::SetEnvironmentVariable(
  "PATH",
  "$env:USERPROFILE\.merlin\bin;$([System.Environment]::GetEnvironmentVariable('PATH','User'))",
  "User"
)
```

---

### Option 2 — Download `.exe` manually

1. Go to the [Releases page](https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest)
2. Download `merlin-windows-amd64.exe`
3. Rename it to `merlin.exe` and place it anywhere on your `PATH`

---

## Docker

The official image is published to GitHub Container Registry on every release.

### Pull the image

```bash
docker pull ghcr.io/arunachalamkalimuthu/merlin-ai-code-review:latest
```

Pin a specific version:

```bash
docker pull ghcr.io/arunachalamkalimuthu/merlin-ai-code-review:v0.1.0
```

### Run a review

```bash
docker run --rm \
  -e GITHUB_TOKEN="$GITHUB_TOKEN" \
  -e ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY" \
  -e GITHUB_ACTIONS="true" \
  -e GITHUB_REPOSITORY="owner/repo" \
  -e GITHUB_SHA="abc123" \
  -e GITHUB_REF="refs/pull/42/merge" \
  ghcr.io/arunachalamkalimuthu/merlin-ai-code-review:latest \
  review
```

### Mount a local diff file

```bash
docker run --rm \
  -v "$(pwd)/changes.diff:/changes.diff" \
  -e ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY" \
  ghcr.io/arunachalamkalimuthu/merlin-ai-code-review:latest \
  review --diff /changes.diff
```

### Docker Compose (bot / webhook mode)

```yaml
# docker-compose.yml
services:
  merlin:
    image: ghcr.io/arunachalamkalimuthu/merlin-ai-code-review:latest
    command: webhook --port 8080
    ports:
      - "8080:8080"
    environment:
      ANTHROPIC_API_KEY: ${ANTHROPIC_API_KEY}
      GITHUB_TOKEN: ${GITHUB_TOKEN}
      MERLIN_GITHUB_SECRET: ${MERLIN_GITHUB_SECRET}
    restart: unless-stopped
```

```bash
docker compose up -d
```

---

## Build from Source

Building from source gives you the latest unreleased code and lets you customise the binary.

### Prerequisites

Install Rust via [rustup](https://rustup.rs):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

Rust **1.75 or newer** is required (the project's MSRV). The `rust-toolchain.toml` file
in the repository pins contributors and CI to **1.85** for reproducibility, but 1.75+ is
sufficient to build a working binary.

### Clone and build

```bash
git clone https://github.com/Arunachalamkalimuthu/merlin-ai-code-review.git
cd merlin-ai-code-review

# Build optimised release binary
cargo build --release

# The binary is at:
./target/release/merlin --help
```

### Install system-wide

```bash
# macOS / Linux
sudo cp target/release/merlin /usr/local/bin/

# Or add the Cargo bin dir to PATH and use cargo install
cargo install --path .
```

### Run tests

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

---

## CI/CD Environments

### GitHub Actions

```yaml
# .github/workflows/review.yml
on:
  pull_request:
    types: [opened, synchronize]

jobs:
  merlin-review:
    runs-on: ubuntu-latest
    permissions:
      pull-requests: write
    steps:
      - uses: actions/checkout@v4
        with: { fetch-depth: 0 }

      - name: Install Merlin
        run: |
          curl -fsSL \
            https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.sh \
            | sh

      - name: Run review
        run: merlin review
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
```

**Cache the binary** to avoid downloading it on every run:

```yaml
      - name: Cache Merlin binary
        uses: actions/cache@v4
        id: cache-merlin
        with:
          path: ~/.local/bin/merlin
          key: merlin-${{ runner.os }}-latest

      - name: Install Merlin
        if: steps.cache-merlin.outputs.cache-hit != 'true'
        run: |
          curl -fsSL \
            https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.sh \
            | sh
```

---

### GitLab CI

```yaml
# .gitlab-ci.yml
merlin-review:
  stage: review
  image: ubuntu:22.04
  before_script:
    - apt-get update -qq && apt-get install -y -qq curl
    - |
      curl -fsSL \
        https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.sh \
        | sh
  script:
    - merlin review
  variables:
    GITLAB_TOKEN: $CI_JOB_TOKEN
    ANTHROPIC_API_KEY: $ANTHROPIC_API_KEY
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
  cache:
    key: merlin-binary
    paths:
      - /usr/local/bin/merlin
```

---

### Bitbucket Pipelines

```yaml
# bitbucket-pipelines.yml
pipelines:
  pull-requests:
    '**':
      - step:
          name: Merlin Review
          image: ubuntu:22.04
          script:
            - apt-get update -qq && apt-get install -y curl
            - curl -fsSL .../install.sh | sh
            - merlin review
```

---

### Azure DevOps

```yaml
# azure-pipelines.yml
trigger: none
pr:
  - main

steps:
  - bash: |
      curl -fsSL \
        https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.sh \
        | sh
      merlin review
    env:
      AZURE_DEVOPS_TOKEN: $(System.AccessToken)
      ANTHROPIC_API_KEY: $(ANTHROPIC_API_KEY)
    displayName: 'Merlin Code Review'
```

---

## Verify Your Installation

```bash
merlin --version
# merlin 0.1.0

merlin --help
```

---

## First Review

Set your AI provider key and run a review against a local diff file — no CI required:

```bash
# 1. Set your key
export ANTHROPIC_API_KEY="sk-ant-..."

# 2. Create a diff from your current branch
git diff main > changes.diff

# 3. Review it
merlin review --diff changes.diff

# 4. Output as JSON (for scripting)
merlin review --diff changes.diff --output json
```

Expected output:

```
[merlin] Parsed 3 changed files
[merlin] Generated 6 review chunks
[merlin] Review complete — 4 comments
```

---

## Uninstall

**Binary installs:**

```bash
# macOS / Linux — whichever path was used
sudo rm /usr/local/bin/merlin
# or
rm ~/.local/bin/merlin
```

**Windows:**

```powershell
Remove-Item "$env:USERPROFILE\.merlin\bin\merlin.exe"
```

**Docker:**

```bash
docker rmi ghcr.io/arunachalamkalimuthu/merlin-ai-code-review:latest
```

**Cargo install:**

```bash
cargo uninstall merlin
```

---

## Troubleshooting

### `merlin: command not found`

The install directory is not on your `PATH`. Add it:

```bash
# ~/.zshrc or ~/.bashrc
export PATH="$HOME/.local/bin:$PATH"
```

Then reload your shell: `source ~/.zshrc`

---

### `permission denied` on install

The installer needs write access to `/usr/local/bin`. Either run with `sudo`:

```bash
sudo sh -c "$(curl -fsSL .../install.sh)"
```

Or install to a user-writable directory:

```bash
MERLIN_INSTALL_DIR="$HOME/.local/bin" curl -fsSL .../install.sh | sh
```

---

### macOS Gatekeeper — "cannot be opened because the developer cannot be verified"

The pre-built binaries are not notarised by Apple. Remove the quarantine attribute:

```bash
xattr -d com.apple.quarantine /usr/local/bin/merlin
```

Or build from source (see [Build from Source](#build-from-source)) to avoid this entirely.

---

### Checksum mismatch

The download may have been interrupted. Delete the file and try again:

```bash
rm -f /tmp/merlin-*
curl -fsSL .../install.sh | sh
```

To skip verification (not recommended):

```bash
MERLIN_NO_VERIFY=1 curl -fsSL .../install.sh | sh
```

---

### `ANTHROPIC_API_KEY` not set

```
error: Environment variable missing: ANTHROPIC_API_KEY
```

Export the key before running Merlin:

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
merlin review --diff changes.diff
```

In CI, add it as a repository secret and pass it via the `env:` block in your workflow.

---

### Platform not detected

```
error: Could not auto-detect CI platform
```

Merlin reads well-known CI environment variables to identify the platform.
If you are running outside of CI, use local mode:

```bash
merlin review --diff changes.diff
```

Or specify the platform explicitly in `merlin.toml`:

```toml
[platform]
type = "github"   # "github" | "gitlab" | "bitbucket" | "azure-devops" | "gitea"
```

---

For further help, open an issue at
<https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/issues>
