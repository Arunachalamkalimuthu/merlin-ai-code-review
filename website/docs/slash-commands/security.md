---
sidebar_position: 7
title: /security
---

# /security

Dedicated security scan focused on the changed code. More thorough than the security checks included in `/review`.

## Usage

```
@merlin /security
```

```bash
merlin run /security
```

## What it checks

- **Secrets exposure** — API keys, tokens, passwords hardcoded in diff
- **OWASP Top 10** — injection, XSS, broken auth, IDOR, SSRF, etc.
- **Authentication & authorisation** — missing checks, privilege escalation vectors
- **Cryptography** — weak algorithms, hardcoded salts, improper key handling
- **Dependency vulnerabilities** — new packages added with known CVEs
- **Input validation** — unvalidated user input reaching sensitive operations
- **Error handling** — stack traces exposed, sensitive data in logs

## Output

Merlin posts inline comments at each finding plus a security summary comment with:

- Total findings by severity
- Risk rating (low / medium / high / critical)
- OWASP categories affected

## Combine with Snyk

For dependency-level CVE scanning, also run `/snyk`:

```
@merlin /security
@merlin /snyk
```
