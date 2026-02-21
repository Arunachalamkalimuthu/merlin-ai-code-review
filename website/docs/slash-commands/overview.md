---
sidebar_position: 1
title: Overview
---

# Slash Commands

Merlin supports 19 slash commands. Trigger them in two ways:

**1. PR comment** — mention `@merlin /command` in any PR/MR comment:
```
@merlin /review
@merlin /ask Is this change thread-safe?
@merlin /spec
```

**2. CLI** — run them directly from CI or your terminal:
```bash
merlin run /review
merlin run /ask "Is this change thread-safe?"
merlin run /spec
```

## All commands

| Command | Description | Output |
|---|---|---|
| [`/review`](./review) | Full code review with inline comments, severity ratings, and a summary | PR inline comments + summary |
| [`/spec`](./spec) | Generate a comprehensive technical specification and set it as the PR description | Updates PR title + description |
| [`/describe`](./describe) | Auto-generate a structured PR title and description from the diff | Updates PR description |
| [`/ask <question>`](./ask) | Q&A about the PR diff | PR comment |
| [`/improve`](./improve) | Inline code suggestion blocks for reviewers to accept with one click | PR suggestion comments |
| [`/security`](./security) | Dedicated security scan — secrets exposure, OWASP Top 10, auth issues | Inline comments + report |
| [`/generate_labels`](./other-commands#generate_labels) | Auto-label based on diff content and size (XS/S/M/L/XL) | PR labels |
| [`/update_changelog`](./other-commands#update_changelog) | Prepend a conventional entry to CHANGELOG.md | File commit |
| [`/add_doc`](./other-commands#add_doc) | Generate missing docstrings and inline comments | PR suggestion comments |
| [`/similar_issue`](./other-commands#similar_issue) | Find related open issues in the same repo | PR comment table |
| [`/test`](./other-commands#test) | Generate unit tests for changed functions | PR comment with test code |
| [`/explain`](./other-commands#explain) | Plain-language walkthrough of what the PR does | PR comment |
| [`/approve`](./other-commands#approve) | AI-assisted review verdict (approve / request changes / comment) | PR review |
| [`/commit_message`](./other-commands#commit_message) | Generate 3 conventional commit message options | PR comment |
| [`/docs [mode]`](./other-commands#docs) | Documentation generator for READMEs, API refs, ADRs, wikis | PR comment or file commit |
| [`/snyk`](./other-commands#snyk) | Scan changed dependencies against the Snyk vulnerability database | PR comment |
| [`/coverage`](./other-commands#coverage) | Analyse test coverage for changed files | PR comment |
| [`/link_jira`](./other-commands#link_jira) | Find related Jira issues and link them to the PR | PR comment |
| [`/link_linear`](./other-commands#link_linear) | Find related Linear issues and link them to the PR | PR comment |
| [`/triage`](./other-commands#triage) | Find similar open issues on CodeTriage for changed packages | PR comment |

## Automatic commands

Some commands run automatically without any mention:

| Trigger | Command | When |
|---|---|---|
| PR opened | `/spec` | Generates a technical spec and sets it as the PR description |
| PR push (if configured) | `/review` | Full review on every commit |

See [GitHub Actions](../getting-started/github-actions) for the workflow setup.
