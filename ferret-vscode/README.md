# Merlin — VS Code Extension

AI-powered code review for VS Code. Self-hosted, bring-your-own-key.

## Requirements

The `merlin` CLI must be installed and available on your PATH (or configured via `merlin.binaryPath`).

```bash
# Install merlin (from a release binary or cargo install)
cargo install --path /path/to/merlin
# or download from GitHub Releases
```

## Features

| Command | Shortcut | Description |
|---------|----------|-------------|
| **Merlin: Review Current File** | — | Review the entire file for bugs, security, style |
| **Merlin: Review Selected Code** | Right-click → Review | Review just the selected code |
| **Merlin: Explain This Code** | Right-click → Explain | Plain-language explanation |
| **Merlin: Suggest Improvements** | Right-click → Improve | Actionable improvement suggestions |
| **Merlin: Security Scan** | Right-click → Security | OWASP/vulnerability scan |
| **Merlin: Generate Tests** | Right-click → Generate Tests | Unit test scaffolding |
| **Merlin: Generate Docs** | — | Docstrings and module docs |
| **Merlin: Ask a Question** | — | Free-form Q&A about code |
| **Merlin: Configure** | — | Open extension settings |

## Configuration

```jsonc
// settings.json
{
  "merlin.binaryPath": "merlin",           // path to merlin binary
  "merlin.provider": "anthropic",          // AI provider
  "merlin.model": "claude-sonnet-4-6",     // model name
  "merlin.showStatusBar": true,            // show status bar item
  "merlin.autoReviewOnSave": false         // auto-review on file save
}
```

### AI providers

| Provider | Setting | Required env var |
|----------|---------|-----------------|
| Anthropic (default) | `"anthropic"` | `ANTHROPIC_API_KEY` |
| OpenAI | `"openai"` | `OPENAI_API_KEY` |
| Claude Code CLI | `"claude-code"` | (run `claude auth login` once) |
| Google Gemini | `"gemini"` | `GEMINI_API_KEY` |
| Ollama (local) | `"ollama"` | — |
| Azure OpenAI | `"azure-openai"` | `AZURE_OPENAI_API_KEY` |
| Amazon Bedrock | `"bedrock"` | `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` |

## Building from source

```bash
cd merlin-vscode
npm install
npm run compile
# To package as .vsix:
npm run package
```
