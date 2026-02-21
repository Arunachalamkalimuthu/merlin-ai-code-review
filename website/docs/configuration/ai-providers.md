---
sidebar_position: 2
title: AI Providers
---

# AI Providers

Merlin supports six AI backends. Switch between them by changing `[ai] provider` in `merlin.toml`.

## Anthropic Claude (default)

Best quality for code review. Claude Sonnet 4.6 is the default.

```toml
[ai]
provider = "anthropic"
model    = "claude-sonnet-4-6"
```

```bash
export ANTHROPIC_API_KEY=sk-ant-...
```

**Available models:** `claude-opus-4-6`, `claude-sonnet-4-6`, `claude-haiku-4-5-20251001`

---

## OpenAI GPT-4o

```toml
[ai]
provider = "openai"
model    = "gpt-4o"
```

```bash
export OPENAI_API_KEY=sk-...
```

**Available models:** `gpt-4o`, `gpt-4o-mini`, `gpt-4-turbo`

:::tip
`OPENAI_API_KEY` is also used for RAG embeddings when `[rag] embedder = "openai"`. A single key serves both purposes.
:::

---

## Claude Code CLI

Use your existing Claude Code subscription — no separate `ANTHROPIC_API_KEY` needed.

```toml
[ai]
provider = "claude-code"
model    = "claude-sonnet-4-6"
```

**Developer machine:**
```bash
claude auth login
```

**CI headless (pass a token):**
```toml
[ai]
provider          = "claude-code"
claude_code_token = ""   # or set CLAUDE_CODE_TOKEN env var
```
```yaml
# GitHub Actions
- run: merlin review
  env:
    CLAUDE_CODE_TOKEN: ${{ secrets.CLAUDE_CODE_TOKEN }}
```

---

## Google Gemini

```toml
[ai]
provider = "gemini"
model    = "gemini-1.5-pro"
```

```bash
export GEMINI_API_KEY=...
```

Get your key from [Google AI Studio](https://aistudio.google.com/).

**Available models:** `gemini-1.5-pro`, `gemini-1.5-flash`, `gemini-2.0-flash`

---

## AWS Bedrock

Run Claude models through your existing AWS infrastructure — no Anthropic account required.

```toml
[ai]
provider       = "bedrock"
model          = "anthropic.claude-sonnet-4-6-20250514-v1:0"
bedrock_region = "us-east-1"
```

```bash
export AWS_ACCESS_KEY_ID=...
export AWS_SECRET_ACCESS_KEY=...
# export AWS_SESSION_TOKEN=...  # for temporary credentials
```

Your IAM role/user needs `bedrock:InvokeModel` permission on the model ARN.

---

## Ollama (local, no API key)

Run a local LLM — fully air-gapped, zero cost.

```toml
[ai]
provider        = "ollama"
model           = "llama3.1"
ollama_base_url = "http://localhost:11434"
```

```bash
ollama serve           # start the server
ollama pull llama3.1   # pull a model
```

**Recommended models for code review:** `llama3.1`, `codestral`, `deepseek-coder-v2`

:::caution
Local models are significantly less capable than Claude or GPT-4o for code review. Expect lower precision and more false positives.
:::

---

## Azure OpenAI

```toml
[ai]
provider                  = "azure-openai"
model                     = "gpt-4o"
azure_openai_endpoint     = "https://myresource.openai.azure.com"
azure_openai_api_version  = "2024-02-01"
```

```bash
export AZURE_OPENAI_API_KEY=...
```
