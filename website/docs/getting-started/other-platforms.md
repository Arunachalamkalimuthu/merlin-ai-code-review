---
sidebar_position: 4
title: Other Platforms
---

# Other Platforms

Merlin auto-detects the CI environment from standard environment variables. No explicit `[platform]` config is needed in most cases.

## Bitbucket Pipelines

```yaml title="bitbucket-pipelines.yml"
pipelines:
  pull-requests:
    '**':
      - step:
          name: Merlin AI Review
          image: ubuntu:22.04
          script:
            - apt-get update -qq && apt-get install -y -qq curl
            - curl -fsSL .../install.sh | sh
            - merlin review
          after-script:
            - echo "Review complete"
```

**Environment variables** (set in *Repository Settings → Repository variables*):

| Variable | Value |
|---|---|
| `BITBUCKET_TOKEN` | A Bitbucket App Password with `pullrequests:write` scope |
| `ANTHROPIC_API_KEY` | Your Anthropic API key |

Merlin detects Bitbucket from `BITBUCKET_PIPELINE_UUID`.

## Azure DevOps

```yaml title="azure-pipelines.yml"
trigger: none
pr:
  - main

jobs:
  - job: MerlinReview
    pool:
      vmImage: ubuntu-latest
    steps:
      - checkout: self
        fetchDepth: 0

      - bash: |
          curl -fsSL .../install.sh | sh
          merlin review
        displayName: Merlin AI Code Review
        env:
          AZURE_DEVOPS_TOKEN: $(System.AccessToken)
          ANTHROPIC_API_KEY: $(ANTHROPIC_API_KEY)
```

Set `ANTHROPIC_API_KEY` as a pipeline variable (not secret, or as a secret variable). `System.AccessToken` is provided automatically.

Merlin detects Azure DevOps from `TF_BUILD`.

## Gitea Actions

```yaml title=".gitea/workflows/review.yml"
on:
  pull_request:
    types: [opened, synchronize]

jobs:
  merlin-review:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with: { fetch-depth: 0 }

      - name: Install Merlin
        run: curl -fsSL .../install.sh | sh

      - name: Review PR
        run: merlin review
        env:
          GITEA_TOKEN: ${{ secrets.GITEA_TOKEN }}
          GITEA_URL: ${{ secrets.GITEA_URL }}
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
```

Merlin detects Gitea Actions from `GITEA_ACTIONS`.

## Manual / local testing

Use `--diff` mode to review a diff file without a platform connection:

```bash
# Generate a diff
git diff main...feature-branch > changes.diff

# Review it locally
merlin review --diff changes.diff

# Output as JSON
merlin review --diff changes.diff --output json | jq '.[] | select(.severity == "critical")'
```

## Forcing a specific platform

If auto-detection fails, set the platform explicitly in `merlin.toml`:

```toml
[platform]
type = "github"   # "github" | "gitlab" | "bitbucket" | "azure-devops" | "gitea"
```
