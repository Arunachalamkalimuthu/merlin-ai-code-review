---
sidebar_position: 8
title: Other Commands
---

# Other Commands

## /generate_labels {#generate_labels}

Automatically applies labels to the PR based on diff content and size.

```
@merlin /generate_labels
```

**Labels applied:**

| Label | Trigger |
|---|---|
| `size/XS` … `size/XL` | Lines changed (0–10 / 11–50 / 51–250 / 251–1000 / 1000+) |
| `security` | Auth, token, secret, key files changed |
| `database` | SQL migrations, schema changes |
| `tests` | Test files added or modified |
| `documentation` | README, docs, markdown files changed |
| `dependencies` | Lock files or package manifests changed |

---

## /update_changelog {#update_changelog}

Prepends a conventional changelog entry to `CHANGELOG.md`.

```
@merlin /update_changelog
```

Format: [Keep a Changelog](https://keepachangelog.com/) — sections for Added, Changed, Fixed, Removed, Security.

---

## /add_doc {#add_doc}

Generates missing docstrings and inline documentation comments for functions, structs, and modules changed in the diff.

```
@merlin /add_doc
```

Output: PR suggestion comments that can be accepted with one click.

---

## /similar_issue {#similar_issue}

Searches open issues in the same repository for issues related to the files or concepts changed in the PR.

```
@merlin /similar_issue
```

Output: A table of related issues with titles, numbers, and relevance scores.

---

## /test {#test}

Generates unit tests for the functions and methods changed in the diff.

```
@merlin /test
```

Output: A PR comment with test code in the same language as the changed files.

---

## /explain {#explain}

Posts a plain-language walkthrough of what the PR does — useful for non-technical reviewers or onboarding.

```
@merlin /explain
```

---

## /approve {#approve}

Issues an AI-assisted review verdict.

```
@merlin /approve
```

Merlin reads the full diff, reviews it, and posts one of:
- ✅ **Approved** — no blocking issues found
- 💬 **Comment** — minor issues noted, not blocking
- 🔴 **Request changes** — blocking issues found (lists them)

:::note
This posts a PR review event, not just a comment. Ensure your workflow has `pull-requests: write` permission.
:::

---

## /commit_message {#commit_message}

Generates three conventional commit message options for the changes in the PR.

```
@merlin /commit_message
```

Output:
```
Option 1: feat(auth): add OAuth2 PKCE flow for mobile clients
Option 2: feat: implement PKCE extension for public client authentication
Option 3: add: OAuth2 authorization code flow with PKCE support
```

---

## /docs {#docs}

Documentation generator with six modes.

```
@merlin /docs readme     # Generate or update README section
@merlin /docs api        # Generate API reference
@merlin /docs adr        # Generate Architecture Decision Record
@merlin /docs module     # Generate module/package docstrings
@merlin /docs wiki       # Generate wiki page
@merlin /docs            # Auto-detect the best type
```

---

## /snyk {#snyk}

Scans changed dependencies against the Snyk vulnerability database.

```
@merlin /snyk
```

**Requires:** `SNYK_TOKEN` environment variable.

---

## /coverage {#coverage}

Analyses test coverage for files changed in the PR.

```
@merlin /coverage
```

**Requires:** A coverage report file (LCOV, Cobertura, or JSON). Configure in `merlin.toml`:

```toml
[coverage]
format      = "lcov"
report_path = "coverage/lcov.info"
threshold   = 80   # fail below 80% coverage
```

---

## /link_jira {#link_jira}

Searches your Jira project for issues related to the PR and posts a table of matches.

```
@merlin /link_jira
```

**Requires:** `JIRA_TOKEN`, and in `merlin.toml`:
```toml
[jira]
base_url    = "https://company.atlassian.net"
project_key = "PROJ"
user_email  = "you@company.com"
```

---

## /link_linear {#link_linear}

Searches Linear for issues related to the PR.

```
@merlin /link_linear
```

**Requires:** `LINEAR_API_KEY`.

---

## /triage {#triage}

Finds similar open issues on [CodeTriage](https://www.codetriage.com/) for packages changed in the PR.

```
@merlin /triage
```
