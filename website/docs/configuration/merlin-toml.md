---
sidebar_position: 1
title: merlin.toml
---

# merlin.toml

Drop a `merlin.toml` file in the root of your repository to configure Merlin. All fields are optional — Merlin works with zero config using sensible defaults.

## Full reference

```toml
# ── AI provider ────────────────────────────────────────────────────────────────
[ai]
# "anthropic" | "openai" | "claude-code" | "gemini" | "bedrock" | "ollama"
provider    = "anthropic"
model       = "claude-sonnet-4-6"
max_tokens  = 4096
temperature = 0.2

# Claude Code CLI (no API key needed)
# claude_code_token = ""

# Ollama
# ollama_base_url = "http://localhost:11434"

# Azure OpenAI
# azure_openai_endpoint    = "https://myresource.openai.azure.com"
# azure_openai_api_version = "2024-02-01"

# AWS Bedrock
# bedrock_region = "us-east-1"

# ── Review behaviour ───────────────────────────────────────────────────────────
[review]
focus        = ["bugs", "security", "style", "performance"]
max_comments = 30      # cap inline comments per review
chunk_lines  = 200     # lines per diff chunk sent to AI
reflect      = false   # enable second-pass comment refinement

# Custom review persona
[review.persona]
# name               = "security-focused"
# system_prompt_extra = "Be extra strict about authentication and authorisation."
# focus_override     = ["security", "bugs"]
# rules              = ["Never approve changes that remove input validation"]

# ── RAG (context-aware reviews) ────────────────────────────────────────────────
[rag]
enabled   = false
embedder  = "ollama"    # "ollama" (local dev) | "openai" (CI/CD)
store     = "local"     # "local" | "memory" | "qdrant" | "chroma" | "pinecone"

embed_model  = "nomic-embed-text"   # for ollama; use "text-embedding-3-small" for openai
collection   = "merlin"
top_k        = 5
min_score    = 0.70
chunk_lines  = 80
local_path   = "merlin-rag.jsonl"
index_extensions = [".rs", ".ts", ".js", ".py", ".go", ".java", ".md"]

# Qdrant
# qdrant_url     = "http://localhost:6333"
# qdrant_api_key = ""

# ChromaDB
# chroma_url = "http://localhost:8000"

# Pinecone
# pinecone_host    = "https://my-index.svc.us-east1.pinecone.io"
# pinecone_api_key = ""

# ── Agent ──────────────────────────────────────────────────────────────────────
[agent]
max_iterations      = 10
max_memory_messages = 50
# memory_file       = ".merlin-memory.jsonl"
default_channel     = "cli"
port                = 8090

# ── Integrations ───────────────────────────────────────────────────────────────
[jira]
# base_url    = "https://company.atlassian.net"
# project_key = "PROJ"
# user_email  = "you@company.com"

[linear]
# team_id = "TEAM_ID"

[coverage]
format      = "lcov"
report_path = "coverage/lcov.info"
threshold   = 0.0   # 0 = disabled; set e.g. 80 to fail below 80% coverage

[snyk]
enabled = false
# org_id = ""   # defaults to token's personal org

[audit]
enabled  = true
log_path = "merlin-audit.jsonl"
```

## Defaults

| Section | Key | Default |
|---|---|---|
| `[ai]` | `provider` | `anthropic` |
| `[ai]` | `model` | `claude-sonnet-4-6` |
| `[ai]` | `max_tokens` | `4096` |
| `[ai]` | `temperature` | `0.2` |
| `[review]` | `max_comments` | `30` |
| `[review]` | `chunk_lines` | `200` |
| `[rag]` | `embedder` | `ollama` |
| `[rag]` | `store` | `local` |
| `[rag]` | `top_k` | `5` |
| `[rag]` | `min_score` | `0.70` |

## Loading order

1. `merlin.toml` in the current working directory
2. Path passed via `--config /path/to/merlin.toml`
3. Hardcoded defaults for any missing fields

Environment variables always take precedence over the config file for secrets (API keys, tokens).
