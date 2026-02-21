---
sidebar_position: 2
title: Embedders
---

# Embedders

An embedder converts text into a dense vector representation so it can be stored and searched in the vector store. Merlin supports two embedders.

## Ollama (default)

Runs locally — free, private, no API key required.

```toml
[rag]
embedder        = "ollama"
embed_model     = "nomic-embed-text"
ollama_base_url = "http://localhost:11434"
```

```bash
ollama serve                      # must be running
ollama pull nomic-embed-text      # download the model once (~274 MB)
```

**Pros:** Free. Code never leaves your machine.
**Cons:** Requires Ollama running locally or as a service. Not suitable for standard CI runners.

### Recommended models

| Model | Dimensions | Size | Notes |
|---|---|---|---|
| `nomic-embed-text` | 768 | 274 MB | Best balance — recommended |
| `mxbai-embed-large` | 1024 | 670 MB | Higher quality, slower |
| `all-minilm` | 384 | 45 MB | Smallest, fastest |

---

## OpenAI

Calls the OpenAI Embeddings API — works from any CI runner.

```toml
[rag]
embedder    = "openai"
embed_model = "text-embedding-3-small"
```

```bash
export OPENAI_API_KEY=sk-...
```

**Pros:** Works anywhere (GitHub Actions, GitLab CI, etc.). Fast. High quality.
**Cons:** Costs money (very cheap). Requires `OPENAI_API_KEY`.

### Recommended models

| Model | Dimensions | Cost per 1M tokens | Notes |
|---|---|---|---|
| `text-embedding-3-small` | 1536 | $0.020 | Recommended — best value |
| `text-embedding-3-large` | 3072 | $0.130 | Higher quality |
| `text-embedding-ada-002` | 1536 | $0.100 | Legacy |

### Typical indexing costs

| Repo size | Estimated cost |
|---|---|
| Small repo (~1 k files) | ~$0.01 |
| Medium repo (~10 k files) | ~$0.10 |
| Large repo (~100 k files) | ~$1.00 |

The index is rebuilt only when files change (use CI caching to avoid re-indexing every run).

:::tip
If you already set `OPENAI_API_KEY` for `provider = "openai"` (review), it also works for `embedder = "openai"` (RAG). One key, two uses.
:::

---

## Fallback behaviour

If `embedder = "openai"` is set but `OPENAI_API_KEY` is missing, Merlin logs a warning and falls back to the Ollama embedder. This means RAG indexing in CI will silently use Ollama if the key isn't configured — ensure the key is set if you want OpenAI embeddings.
