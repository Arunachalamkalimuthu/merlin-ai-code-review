---
sidebar_position: 3
title: GitLab CI
---

# GitLab CI

Works with **GitLab.com** and **self-hosted GitLab** instances.

## Minimal setup

```yaml title=".gitlab-ci.yml"
merlin-review:
  image: ubuntu:22.04
  stage: review
  script:
    - apt-get update -qq && apt-get install -y -qq curl
    - curl -fsSL
        https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.sh
        | MERLIN_INSTALL_DIR=/usr/local/bin sh
    - merlin review
  variables:
    GITLAB_TOKEN: $CI_JOB_TOKEN
    ANTHROPIC_API_KEY: $ANTHROPIC_API_KEY
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
```

**Required CI/CD variables** (set in *Settings → CI/CD → Variables*):

| Variable | Value |
|---|---|
| `ANTHROPIC_API_KEY` | Your Anthropic API key |
| `CI_JOB_TOKEN` | Set automatically by GitLab — no action needed |

## With binary caching

```yaml title=".gitlab-ci.yml"
variables:
  MERLIN_VERSION: "latest"

.merlin-base:
  image: ubuntu:22.04
  cache:
    key: merlin-binary-$CI_COMMIT_REF_SLUG
    paths:
      - .merlin/
  before_script:
    - apt-get update -qq && apt-get install -y -qq curl
    - mkdir -p .merlin/bin
    - |
      if [ ! -f .merlin/bin/merlin ]; then
        curl -fsSL .../install.sh | MERLIN_INSTALL_DIR=.merlin/bin sh
      fi
    - export PATH="$PWD/.merlin/bin:$PATH"

merlin-spec:
  extends: .merlin-base
  stage: review
  script:
    - merlin run /spec
  variables:
    GITLAB_TOKEN: $CI_JOB_TOKEN
    ANTHROPIC_API_KEY: $ANTHROPIC_API_KEY
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
      changes:
        - "**/*"
      when: on_success

merlin-review:
  extends: .merlin-base
  stage: review
  cache:
    - key: merlin-binary-$CI_COMMIT_REF_SLUG
      paths: [.merlin/]
    - key: merlin-rag-$CI_COMMIT_SHORT_SHA
      paths: [merlin-rag.jsonl]
      policy: pull-push
  script:
    - test -f merlin-rag.jsonl || merlin rag index .
    - merlin review
  variables:
    GITLAB_TOKEN: $CI_JOB_TOKEN
    ANTHROPIC_API_KEY: $ANTHROPIC_API_KEY
    OPENAI_API_KEY: $OPENAI_API_KEY
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
```

## Self-hosted GitLab

Point Merlin at your GitLab instance with the `GITLAB_URL` environment variable:

```yaml
variables:
  GITLAB_URL: https://gitlab.company.com
  GITLAB_TOKEN: $CI_JOB_TOKEN
  ANTHROPIC_API_KEY: $ANTHROPIC_API_KEY
```

No other changes are needed — Merlin detects `GITLAB_CI` and uses `GITLAB_URL` automatically.

## Bot mode (webhook)

Deploy Merlin as a persistent service and configure a **GitLab webhook** (Project → Settings → Webhooks) to send Note Hook events to `http://your-server:8080/webhook/gitlab`.

```bash
GITLAB_TOKEN=... ANTHROPIC_API_KEY=... MERLIN_GITLAB_SECRET=... \
  merlin webhook --port 8080
```

Users can then trigger commands by commenting on any MR: `@merlin /review`
