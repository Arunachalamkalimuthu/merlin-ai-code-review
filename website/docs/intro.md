---
slug: /
sidebar_position: 1
title: Introduction
---

# Merlin 🧙

**Self-hosted AI code review for GitHub, GitLab, Bitbucket, Azure DevOps, and Gitea.**

Merlin is open-source, bring-your-own-key, and designed so that your code never leaves your own infrastructure. It parses PR/MR diffs, sends them to a configurable AI provider, and posts inline review comments plus a summary back to the PR/MR.

## What Merlin does

- **Reviews every PR** with inline comments, severity ratings, and a summary — automatically on push, or on-demand via `@merlin /review`
- **Generates technical specifications** the moment a PR is opened, giving reviewers instant context
- **Runs 19 slash commands** — from security scans to changelog updates to unit test generation
- **Maintains a RAG index** of your codebase so AI comments reference real context, not just the diff
- **Operates as an autonomous agent** — give it a task in Slack, Discord, or the CLI and it plans and executes a multi-step workflow

## Comparison

| Feature | Merlin | CodeRabbit | Qodo |
|---|---|---|---|
| Self-hosted | ✅ | ❌ | ❌ |
| Bring-your-own-key | ✅ | ❌ | ❌ |
| Open-source | ✅ (MIT) | ❌ | ❌ |
| Code leaves your infra | ❌ Never | ✅ Their servers | ✅ Their servers |
| RAG codebase indexing | ✅ | ✅ | ✅ |
| Autonomous agent | ✅ | ❌ | ❌ |
| Slash commands | 19 | ~6 | ~8 |
| AI provider choice | 6 | Fixed | Fixed |

## Architecture overview

```
CLI (clap)
  ├── ReviewEngine
  │     ├── PlatformClient  (GitHub | GitLab | Bitbucket | Azure DevOps | Gitea)
  │     ├── DiffParser → Vec<FileDiff>  (token-aware, security-ranked)
  │     ├── AiProvider  (Anthropic | OpenAI | Claude Code | Gemini | Bedrock | Ollama)
  │     └── RagPipeline  (optional)
  │           ├── Embedder  (OllamaEmbedder | OpenAiEmbedder)
  │           └── VectorStore  (local | qdrant | chroma | pinecone)
  │
  ├── ToolRouter  (19 slash commands)
  │     └── Webhook server (axum) → dispatches commands from PR comments
  │
  └── AgentRuntime  (ReAct loop)
        ├── AgentMemory  (ring-buffer + JSONL persistence)
        └── AgentChannel  (CLI | Slack | Discord)
```

## Next steps

- [Install Merlin](./getting-started/installation) and add it to your pipeline in 60 seconds
- [Configure your AI provider](./configuration/ai-providers)
- [Explore all slash commands](./slash-commands/overview)
- [Set up RAG indexing](./rag/overview)
