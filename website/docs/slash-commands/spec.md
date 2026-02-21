---
sidebar_position: 3
title: /spec
---

# /spec

Generates a comprehensive **Technical Specification** for the PR and sets it as the PR description. Runs automatically when a PR is first opened (if you use the provided workflow).

## Usage

```
@merlin /spec
```

```bash
merlin run /spec
```

## What the spec includes

The AI generates a structured document with up to 10 sections:

1. **Overview** — what the PR does in 2–3 sentences
2. **Problem statement** — why this change is needed
3. **Technical approach** — key design decisions and trade-offs
4. **Changes** — file-by-file breakdown of what changed and why
5. **API changes** — new/modified endpoints, request/response shapes
6. **Data model changes** — schema migrations, new fields, renamed columns
7. **Dependencies** — new libraries added and why
8. **Testing strategy** — how the changes are tested (or should be)
9. **Rollout notes** — feature flags, migration steps, backwards-compatibility concerns
10. **Open questions** — unresolved decisions flagged for reviewers

## Automatic trigger

Add the `merlin-spec` job to your workflow so the spec is generated the moment a PR is opened:

```yaml title=".github/workflows/review.yml"
merlin-spec:
  name: Generate Technical Spec
  runs-on: ubuntu-latest
  if: github.event.action == 'opened'
  permissions:
    pull-requests: write
    contents: read
  steps:
    - uses: actions/checkout@v4
      with: { fetch-depth: 0 }
    - run: merlin run /spec
      env:
        GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
```

## Output format

The spec is written as Markdown and set as the PR description. The `# H1` title becomes the PR title; the body (everything after the H1) becomes the description.

Example output:

```markdown
# feat: add OAuth2 PKCE flow for mobile clients

## Overview
Implements the OAuth2 Authorization Code flow with PKCE extension to
support secure authentication for the iOS and Android apps.

## Problem Statement
The current implicit flow is deprecated and insecure for native apps.
PKCE allows public clients to authenticate without a client secret.

## Technical Approach
...
```
