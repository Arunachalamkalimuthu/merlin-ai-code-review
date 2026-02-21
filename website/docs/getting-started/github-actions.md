---
sidebar_position: 2
title: GitHub Actions
---

# GitHub Actions

## Minimal setup (2 minutes)

Add this file to your repository:

```yaml title=".github/workflows/review.yml"
on:
  pull_request:
    types: [opened, synchronize, reopened]

jobs:
  merlin-review:
    name: AI Code Review
    runs-on: ubuntu-latest
    permissions:
      pull-requests: write
      contents: read
    steps:
      - uses: actions/checkout@v4
        with: { fetch-depth: 0 }

      - name: Install Merlin
        run: |
          curl -fsSL \
            https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.sh \
            | MERLIN_INSTALL_DIR=~/.local/bin sh

      - name: Review PR
        run: ~/.local/bin/merlin review
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
```

**Required secrets** (set in *Settings → Secrets and variables → Actions*):

| Secret | Where to get it |
|---|---|
| `ANTHROPIC_API_KEY` | [console.anthropic.com](https://console.anthropic.com) |
| `GITHUB_TOKEN` | Provided automatically — no action needed |

## Full setup with caching and auto-spec

```yaml title=".github/workflows/review.yml"
name: Merlin AI Code Review

on:
  pull_request:
    types: [opened, synchronize, reopened]

jobs:
  # Generate a technical spec when the PR is first opened
  merlin-spec:
    name: Generate Technical Spec
    runs-on: ubuntu-latest
    if: github.event.action == 'opened'
    permissions:
      pull-requests: write
      contents: read
    env:
      MERLIN_VERSION: latest
    steps:
      - uses: actions/checkout@v4
        with: { fetch-depth: 0 }

      - name: Cache Merlin binary
        id: cache-merlin
        uses: actions/cache@v4
        with:
          path: ~/.local/bin/merlin
          key: merlin-${{ env.MERLIN_VERSION }}-linux-amd64

      - name: Install Merlin
        if: steps.cache-merlin.outputs.cache-hit != 'true'
        run: |
          mkdir -p ~/.local/bin
          curl -fsSL \
            https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.sh \
            | MERLIN_INSTALL_DIR=~/.local/bin sh

      - name: Generate technical specification
        run: ~/.local/bin/merlin run /spec
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}

  # Full code review on every push
  merlin-review:
    name: AI Code Review
    runs-on: ubuntu-latest
    permissions:
      pull-requests: write
      contents: read
    env:
      MERLIN_VERSION: latest
    steps:
      - uses: actions/checkout@v4
        with: { fetch-depth: 0 }

      - name: Cache Merlin binary
        id: cache-merlin
        uses: actions/cache@v4
        with:
          path: ~/.local/bin/merlin
          key: merlin-${{ env.MERLIN_VERSION }}-linux-amd64

      - name: Install Merlin
        if: steps.cache-merlin.outputs.cache-hit != 'true'
        run: |
          mkdir -p ~/.local/bin
          curl -fsSL .../install.sh | MERLIN_INSTALL_DIR=~/.local/bin sh

      # Optional: RAG context-aware reviews (requires merlin.toml with [rag] enabled = true)
      - name: Cache RAG index
        uses: actions/cache@v4
        with:
          path: merlin-rag.jsonl
          key: merlin-rag-${{ hashFiles('src/**', 'lib/**') }}
          restore-keys: merlin-rag-

      - name: Build RAG index (on cache miss)
        run: test -f merlin-rag.jsonl || ~/.local/bin/merlin rag index .
        env:
          OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}

      - name: Review PR
        run: ~/.local/bin/merlin review
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
```

## Pinning to a version

Replace `latest` with a specific release tag to avoid unexpected changes:

```yaml
env:
  MERLIN_VERSION: v1.2.0
```

## Bot mode (webhook)

For slash commands triggered by PR comments (`@merlin /review`), deploy Merlin as a long-running webhook server instead:

```yaml
- name: Start Merlin webhook
  run: merlin webhook --port 8080 &
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
    ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
    MERLIN_GITHUB_SECRET: ${{ secrets.MERLIN_WEBHOOK_SECRET }}
```

See [Bot Mode](../bot-mode/overview) for the full guide.
