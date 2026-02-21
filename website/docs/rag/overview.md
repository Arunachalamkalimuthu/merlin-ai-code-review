---
sidebar_position: 1
title: Overview
---

# RAG (Context-Aware Reviews)

RAG (Retrieval-Augmented Generation) lets Merlin search your codebase for relevant context before reviewing each diff chunk. Instead of seeing only the changed lines, the AI also sees related files, past review patterns, and architectural context — resulting in more accurate and actionable comments.

## How it works

```
Index time (once):
  Source files ──► embed ──► vector store (merlin-rag.jsonl / Qdrant / etc.)

Review time (every PR):
  Diff chunk ──► embed ──► vector search ──► top-K similar docs
  top-K docs ─────────────────────────────────────────────────► AI prompt
```

## Quick start (local dev)

```toml title="merlin.toml"
[rag]
enabled  = true
embedder = "ollama"    # free, local
store    = "local"     # zero infra — JSONL file
```

```bash
ollama pull nomic-embed-text    # one-time model download
merlin rag index .              # index your codebase
merlin review                   # reviews now include context
```

## Quick start (CI/CD)

```toml title="merlin.toml"
[rag]
enabled     = true
embedder    = "openai"                   # no Ollama needed in CI
embed_model = "text-embedding-3-small"
store       = "local"
```

```yaml title=".github/workflows/review.yml"
- uses: actions/cache@v4
  with:
    path: merlin-rag.jsonl
    key: merlin-rag-${{ hashFiles('src/**', 'lib/**') }}
    restore-keys: merlin-rag-

- run: test -f merlin-rag.jsonl || merlin rag index .
  env:
    OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
```

The index is rebuilt only when your source files change (cache key = hash of source tree). Cost to index a 10 k-file repo: **~$0.10** in OpenAI embedding credits.

## CLI commands

```bash
merlin rag index .              # index current directory
merlin rag index src/           # index a subdirectory
merlin rag search "auth bypass" # test a query
merlin rag search "SQL" -k 10   # return up to 10 results
merlin rag count                # documents in the index
merlin rag clear                # delete all indexed data
```

## Next steps

- [Choose an embedder](./embedders) — Ollama vs OpenAI
- [Choose a vector store](./vector-stores) — local file vs Qdrant vs Pinecone
- [CI caching guide](./ci-caching) — cache the index between pipeline runs
