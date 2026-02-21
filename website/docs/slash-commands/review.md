---
sidebar_position: 2
title: /review
---

# /review

Full AI code review of the PR diff. Posts inline comments at the relevant file and line, then a summary comment with overall findings.

## Usage

```
@merlin /review
```

```bash
merlin review              # full CI review (auto-detects platform)
merlin run /review         # via run subcommand
merlin review --diff a.diff --output json   # local, no platform posting
```

## What it reviews

Each comment includes:

- **Severity** — `critical` / `high` / `medium` / `low` / `info`
- **Category** — `bug` / `security` / `style` / `performance`
- **Title** — short description
- **Body** — detailed explanation
- **Suggestion** — optional code fix (shown as an inline GitHub suggestion block)

## Summary

After posting all inline comments, Merlin posts a summary with:

- Total issues by severity
- Files reviewed
- Size label (XS → XL based on lines changed)
- Whether tests were included, migrations detected, or secrets-risk files touched

## Configuration

```toml
[review]
focus        = ["bugs", "security", "style", "performance"]
max_comments = 30       # cap per review
chunk_lines  = 200      # lines per AI call
reflect      = false    # enable second-pass refinement
```

### Persona override

```toml
[review.persona]
name               = "strict-security"
system_prompt_extra = "Pay extra attention to SQL injection and SSRF vectors."
focus_override     = ["security", "bugs"]
rules              = [
  "Flag any use of `unwrap()` in production code as a high-severity bug",
  "Always check for missing rate limiting on public endpoints",
]
```

## Token budget

Merlin ranks files by security risk before sending to the AI:

1. **Critical** — auth, tokens, secrets, keys, passwords, RBAC
2. **High** — application source code
3. **Medium** — tests, fixtures, mocks
4. **Low** — lock files, generated code, docs, assets

Files are processed in priority order until the token budget is exhausted. Low-priority files are dropped first.
