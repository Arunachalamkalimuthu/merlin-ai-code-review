---
sidebar_position: 4
title: CI Caching
---

# Caching the RAG Index in CI

Indexing your codebase on every PR run would be slow and expensive. Instead, cache `merlin-rag.jsonl` between runs and only rebuild when source files change.

## GitHub Actions

```yaml
- name: Cache RAG index
  uses: actions/cache@v4
  with:
    path: merlin-rag.jsonl
    # Key = hash of source files — rebuilds when code changes
    key: merlin-rag-${{ hashFiles('src/**', 'lib/**', '*.go', '*.py', '*.ts') }}
    # Fall back to any previous index on key miss
    restore-keys: merlin-rag-

- name: Build RAG index (cache miss only)
  run: test -f merlin-rag.jsonl || merlin rag index .
  env:
    OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
```

### Required merlin.toml

```toml
[rag]
enabled     = true
embedder    = "openai"
embed_model = "text-embedding-3-small"
store       = "local"
```

## GitLab CI

```yaml
merlin-review:
  cache:
    - key: merlin-binary
      paths: [.merlin/]
    - key: merlin-rag-$CI_COMMIT_SHORT_SHA
      paths: [merlin-rag.jsonl]
      policy: pull-push
  script:
    - test -f merlin-rag.jsonl || merlin rag index .
    - merlin review
  variables:
    OPENAI_API_KEY: $OPENAI_API_KEY
    GITLAB_TOKEN: $CI_JOB_TOKEN
    ANTHROPIC_API_KEY: $ANTHROPIC_API_KEY
```

## How the cache key works

```
key: merlin-rag-${{ hashFiles('src/**', 'lib/**') }}
```

- `hashFiles()` computes a SHA-256 of all matching files
- If any source file changes, the hash changes → cache miss → index rebuilds
- `restore-keys: merlin-rag-` means a stale index (from a previous run) is used during the rebuild, so the first push after a code change still gets RAG context

## Cost summary

| Event | Action | Cost |
|---|---|---|
| Cache hit | Use existing index | $0 |
| Cache miss, 1k files | Rebuild with OpenAI | ~$0.01 |
| Cache miss, 10k files | Rebuild with OpenAI | ~$0.10 |
| Cache miss, local Ollama | Rebuild | $0 (but Ollama must be available) |

## Using Qdrant instead

If you run Qdrant as a service, you don't need file caching — the index persists in Qdrant between runs:

```toml
[rag]
enabled    = true
embedder   = "openai"
store      = "qdrant"
qdrant_url = "http://qdrant.internal:6333"
```

Merlin will only index files not already in Qdrant (upsert semantics).
