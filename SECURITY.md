# Security Policy

## Supported Versions

| Version | Supported |
|---|---|
| 0.1.x (latest) | ✅ |
| < 0.1.0 | ❌ |

We support the latest release only. Please update to the latest version before reporting a vulnerability.

---

## Reporting a Vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Please report security issues privately so we can address them before they are disclosed publicly.

### How to report

**Option 1 — GitHub Private Vulnerability Reporting (preferred)**

1. Go to the [Security tab](https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/security/advisories/new) of this repository
2. Click **"Report a vulnerability"**
3. Fill in the details and submit

**Option 2 — Email**

Send a detailed report to the maintainer via GitHub's contact form at:
[github.com/Arunachalamkalimuthu](https://github.com/Arunachalamkalimuthu)

### What to include

A useful report includes:

- A clear description of the vulnerability
- Steps to reproduce
- The component affected (AI provider, platform client, RAG pipeline, webhook handler, etc.)
- Potential impact (data exposure, token leakage, RCE, etc.)
- Suggested fix (optional but appreciated)

---

## Response Timeline

| Stage | Target |
|---|---|
| Acknowledgement | Within 48 hours |
| Triage and severity assessment | Within 5 business days |
| Fix and patch release | Depends on severity (see below) |
| Public disclosure | After patch ships |

### Severity SLA

| Severity | Fix target |
|---|---|
| Critical (CVSS 9.0–10.0) | 7 days |
| High (CVSS 7.0–8.9) | 14 days |
| Medium (CVSS 4.0–6.9) | 30 days |
| Low (CVSS < 4.0) | Next minor release |

---

## Security Design Notes

### API key handling

- API keys are **never logged** at any log level
- Keys are read from environment variables — never stored in `merlin.toml` (config files are for non-secret settings)
- Keys are not included in review prompts or posted to VCS platforms

### Webhook signature verification

When running in bot/agent mode, Merlin verifies the HMAC-SHA256 signature on all incoming webhook payloads:

- **GitHub** — `X-Hub-Signature-256` header verified against `MERLIN_GITHUB_SECRET`
- **GitLab** — `X-Gitlab-Token` header verified against `MERLIN_GITLAB_SECRET`

Requests with missing or invalid signatures are rejected with `401 Unauthorized`.

### Diff handling

- Diffs are processed in memory and never written to disk
- Diffs are sent to the configured AI provider over HTTPS using `rustls` (no native TLS, no system CA trust issues)
- RAG index files (`merlin-rag.jsonl`) may contain code snippets — treat them with the same sensitivity as your source code

### Dependency security

- Dependencies are pinned in `Cargo.lock`
- Automated dependency updates via Dependabot (see `.github/dependabot.yml`)
- `cargo audit` runs in CI on every push

### Docker images

- Images are built from scratch with a minimal base
- No shell, no package manager in the final image
- Published to GHCR with attestations (SLSA Level 2)

---

## Known Limitations

- Merlin trusts the diff content returned by the VCS platform API. A compromised VCS token could result in Merlin processing malicious diffs.
- The local RAG JSONL store (`merlin-rag.jsonl`) is not encrypted at rest. Avoid committing it to repositories.
- Ollama connections (`http://localhost:11434`) are plaintext by default. Do not expose Ollama on a public interface.

---

## Disclosure Policy

We follow [Coordinated Vulnerability Disclosure (CVD)](https://cheatsheetseries.owasp.org/cheatsheets/Vulnerability_Disclosure_Cheat_Sheet.html). After a fix is released, we will:

1. Publish a GitHub Security Advisory
2. Note the fix in `CHANGELOG.md`
3. Credit the reporter (unless they prefer to remain anonymous)
