---
sidebar_position: 8
title: Contributing
---

# Contributing to Merlin

Thank you for your interest in contributing! Merlin is open-source and welcomes bug fixes, features, documentation improvements, and more.

## Development setup

### Prerequisites

- Rust 1.75+
- Git
- (Optional) Ollama for RAG tests
- (Optional) Docker for integration tests

### Clone and build

```bash
git clone https://github.com/you/merlin.git
cd merlin
cargo build
cargo test
```

### Run from source

```bash
cargo run -- review --help
```

## Project structure

```
ferret/
├── src/
│   ├── main.rs           # CLI entrypoint (clap)
│   ├── lib.rs            # Library root, module declarations
│   ├── config.rs         # Config structs + TOML/env parsing
│   ├── error.rs          # Unified error type (thiserror)
│   ├── diff/
│   │   └── parser.rs     # Unified diff parser → FileDiff structs
│   ├── ai/
│   │   ├── mod.rs        # AiProvider trait + factory
│   │   ├── anthropic.rs  # Claude provider
│   │   └── openai.rs     # OpenAI provider
│   ├── platform/
│   │   ├── mod.rs        # PlatformClient trait + NoOpPlatform
│   │   ├── github.rs     # GitHub REST API client
│   │   └── gitlab.rs     # GitLab REST API client
│   ├── rag/
│   │   ├── mod.rs        # RAG pipeline builder
│   │   ├── embedder.rs   # OllamaEmbedder, OpenAiEmbedder
│   │   ├── store/        # Vector stores (local, qdrant, etc.)
│   │   └── indexer.rs    # File crawler + indexer
│   ├── review/
│   │   └── engine.rs     # ReviewEngine: orchestration logic
│   ├── slash/
│   │   └── mod.rs        # Slash command handlers
│   └── webhook/
│       └── mod.rs        # Webhook listener (agent mode)
├── tests/                # Integration tests
├── website/              # Docusaurus documentation
└── Cargo.toml
```

## Making changes

### Bug fixes

1. Open an issue describing the bug (or find an existing one)
2. Create a branch: `git checkout -b fix/issue-description`
3. Write a test that reproduces the bug
4. Fix the bug
5. Verify: `cargo test && cargo clippy -- -D warnings`
6. Open a pull request

### New features

1. Open a discussion or issue first to align on the approach
2. Create a branch: `git checkout -b feat/feature-name`
3. Implement the feature with tests
4. Update documentation in `website/docs/`
5. Run the full test suite
6. Open a pull request

## Code style

- Follow standard Rust idioms and `rustfmt` formatting
- Run `cargo fmt` before committing
- Run `cargo clippy -- -D warnings` — no warnings allowed
- Prefer `thiserror` for error types, `anyhow` in binaries
- Use `tracing` for logging (not `println!`)
- Write tests for non-trivial logic

```bash
# Before opening a PR
cargo fmt
cargo clippy -- -D warnings
cargo test
```

## Testing

### Unit tests

```bash
cargo test
```

### Specific test

```bash
cargo test diff::parser
```

### Integration tests

```bash
cargo test --test '*'
```

### Local review test

```bash
# Test against a real diff file
cargo run -- review --diff tests/fixtures/sample.diff --output json
```

## Adding a new AI provider

1. Create `src/ai/myprovider.rs` implementing the `AiProvider` trait:

```rust
use async_trait::async_trait;
use crate::{error::Result, review::ReviewComment};
use super::{AiProvider, ReviewContext};

pub struct MyProvider { /* fields */ }

#[async_trait]
impl AiProvider for MyProvider {
    async fn review(&self, ctx: &ReviewContext) -> Result<Vec<ReviewComment>> {
        // call API, parse response
        todo!()
    }
}
```

2. Add a variant to `ProviderType` in `config.rs`
3. Wire it up in `ai/mod.rs` factory function
4. Add documentation in `website/docs/configuration/ai-providers.md`

## Adding a new vector store

1. Create `src/rag/store/mystore.rs` implementing the `VectorStore` trait
2. Add a variant to `VectorStoreType` in `config.rs`
3. Wire it up in `rag/mod.rs` `build_pipeline()`
4. Document in `website/docs/rag/vector-stores.md`

## Documentation

The docs live in `website/` and are built with Docusaurus v3.

```bash
cd website
npm install
npm start        # local dev server at http://localhost:3000
npm run build    # production build
```

When adding a new page:
1. Create the `.md` file in the appropriate `docs/` subdirectory
2. Add `sidebar_position` frontmatter
3. If it's a new section, add `_category_.json`
4. Update `sidebars.js` if needed

## Pull request checklist

- [ ] Tests pass: `cargo test`
- [ ] No clippy warnings: `cargo clippy -- -D warnings`
- [ ] Code formatted: `cargo fmt --check`
- [ ] Documentation updated (if applicable)
- [ ] PR description explains the change and motivation

## Reporting bugs

Please include:
- Merlin version (`merlin --version`)
- Operating system
- Minimal `merlin.toml` config
- Steps to reproduce
- Expected vs actual behavior
- Relevant logs (run with `RUST_LOG=debug`)

Open an issue at: [github.com/you/merlin/issues](https://github.com/you/merlin/issues)

## License

Merlin is licensed under the MIT License. By contributing, you agree that your contributions will be licensed under the same terms.
