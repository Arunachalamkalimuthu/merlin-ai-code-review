---
sidebar_position: 5
title: /ask
---

# /ask

Ask any question about the PR diff and get a direct answer posted as a comment.

## Usage

```
@merlin /ask Is this change thread-safe?
@merlin /ask What is the time complexity of the new sorting function?
@merlin /ask Are there any edge cases not covered by the tests?
```

```bash
merlin run /ask "Is this change thread-safe?"
```

The argument after `/ask` is the full question — no quotes needed in PR comments.

## Examples

```
@merlin /ask Does this break backwards compatibility?
@merlin /ask Can you explain what the new middleware does?
@merlin /ask Is there a risk of SQL injection in the new query?
@merlin /ask What happens if the external API returns a 503?
```

Merlin answers in the context of the full diff, not just a single file.
