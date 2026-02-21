---
sidebar_position: 6
title: /improve
---

# /improve

Posts inline **code suggestion blocks** that reviewers can accept with a single click directly in the GitHub/GitLab UI.

## Usage

```
@merlin /improve
```

```bash
merlin run /improve
```

## How it works

Merlin reviews the diff and generates concrete code rewrites for each issue it finds. Each suggestion is posted as a GitHub/GitLab suggestion block:

````
```suggestion
// improved version here
```
````

Reviewers can click **Apply suggestion** in the PR to commit the fix immediately.

## Difference from /review

| | `/review` | `/improve` |
|---|---|---|
| Output | Explanatory comments | Actionable code rewrites |
| Reviewer action | Read and fix manually | One-click accept |
| Best for | Understanding issues | Quickly applying fixes |
