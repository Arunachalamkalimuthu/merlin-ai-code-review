---
sidebar_position: 3
title: Vector Stores
---

# Vector Stores

The vector store persists the embeddings and handles similarity search. Choose based on your scale and infrastructure requirements.

## local (default)

Zero infrastructure — stores embeddings in a JSONL flat file with brute-force cosine similarity.

```toml
[rag]
store      = "local"
local_path = "merlin-rag.jsonl"   # default
```

**Best for:** Repos up to ~5 k files. CI/CD with file caching.
**Pros:** No setup, no services, cacheable in CI.
**Cons:** Linear search — slow for large repos.

---

## memory

Ephemeral in-memory store — data is lost when Merlin exits.

```toml
[rag]
store = "memory"
```

**Best for:** Testing and development.

---

## Qdrant

High-performance vector database. Self-hosted or [Qdrant Cloud](https://cloud.qdrant.io/).

```toml
[rag]
store          = "qdrant"
qdrant_url     = "http://localhost:6333"
# qdrant_api_key = ""   # required for Qdrant Cloud
```

**Self-hosted:**
```bash
docker run -p 6333:6333 qdrant/qdrant
```

**Qdrant Cloud:**
```toml
[rag]
store          = "qdrant"
qdrant_url     = "https://xyz.us-east.aws.cloud.qdrant.io"
qdrant_api_key = ""   # or set QDRANT_API_KEY env var
```

**Best for:** Production self-hosted deployments. Large repos.

---

## ChromaDB

Open-source vector database.

```toml
[rag]
store      = "chroma"
chroma_url = "http://localhost:8000"
```

```bash
docker run -p 8000:8000 chromadb/chroma
```

**Best for:** Open-source alternative to Qdrant. Self-hosted.

---

## Pinecone

Managed cloud vector database — no infrastructure to run.

```toml
[rag]
store         = "pinecone"
pinecone_host = "https://my-index-xyz.svc.us-east1.pinecone.io"
# pinecone_api_key = ""   # or set PINECONE_API_KEY env var
```

**Best for:** Teams that want fully managed infrastructure.

---

## Choosing a store

| Scenario | Recommended store |
|---|---|
| Getting started, small repo | `local` |
| CI/CD with caching | `local` |
| Production, large repo, self-hosted | `qdrant` |
| Production, managed cloud | `pinecone` |
| Testing | `memory` |
