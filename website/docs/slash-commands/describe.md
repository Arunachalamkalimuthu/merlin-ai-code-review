---
sidebar_position: 4
title: /describe
---

# /describe

Auto-generates a structured PR title and description from the diff. Unlike `/spec`, `/describe` produces a shorter summary focused on **what changed**, not a full engineering spec.

## Usage

```
@merlin /describe
```

```bash
merlin run /describe
```

## Output

Merlin updates the PR title and description with a JSON-structured summary:

```json
{
  "title": "fix: correct null check in user authentication",
  "description": "## Summary\nFixes a NullPointerException thrown when ...\n\n## Changes\n..."
}
```

If the AI response isn't valid JSON, Merlin falls back to the existing PR title and posts the raw text as the description.

## Difference from /spec

| | `/describe` | `/spec` |
|---|---|---|
| Length | Short (1–2 paragraphs) | Long (up to 10 sections) |
| Focus | What changed | Why + how + trade-offs |
| Best for | Simple PRs, bug fixes | Features, refactors, migrations |
