# Release Notes — v0.2.0

## Highlights

This release introduces three major intelligence features that make Merlin smarter, less noisy, and more visual — addressing the top pain points teams face with AI code review tools.

---

### Custom Rules Engine (`.merlin-rules.yaml`)

Define team-specific review rules that Merlin enforces on every PR. Rules support:

- **Regex patterns** — match against diff content (e.g. flag `unwrap()`, detect SQL string concatenation)
- **Natural-language directives** — injected into the AI system prompt (e.g. "All public API functions must handle errors")
- **Path-scoped rules** — restrict rules to specific file globs (e.g. `src/auth/**`)
- **Configurable severity** — set the severity level for each rule

```yaml
# .merlin-rules.yaml
rules:
  - name: no-unwrap
    pattern: "unwrap\\(\\)"
    severity: high
    message: "Avoid unwrap() in production code — use ? or expect() with context"

  - name: auth-review
    path_match: "src/auth/**"
    directive: "Flag any changes to authentication logic as Critical severity"
```

Once a team encodes their standards, Merlin enforces them consistently on every review.

---

### Adaptive Feedback Learning

Merlin learns from your team's reactions to review comments. Comment patterns that are consistently rejected are auto-suppressed, reducing noise without manual configuration.

- React to review comments with 👍 (accept) or 👎 (reject)
- Patterns with 5+ events and >70% rejection rate are auto-suppressed
- Feedback is stored locally in JSONL format — commit it for shared learning or `.gitignore` it for per-environment tuning
- Run `/feedback` to see learning status and suppressed patterns

```toml
# merlin.toml
[review]
feedback_learning = true
feedback_path     = ".merlin-feedback.jsonl"
```

---

### PR Architecture Diagrams (`/diagram`)

Auto-generate Mermaid architecture diagrams from PR diffs. The diagram shows which modules are affected, how changed files relate to each other, and the data flow between components.

```bash
merlin run /diagram
# or from a PR comment:
@merlin /diagram
```

The AI analyses the diff, groups files by module, and produces a clean Mermaid diagram posted as a PR comment — helping reviewers understand scope and impact at a glance.

---

## New Slash Commands

| Command | Description |
|---------|-------------|
| `/diagram` | Generate a Mermaid architecture diagram of PR changes |
| `/feedback` | Show adaptive feedback learning status and suppressed patterns |

Total slash commands: **22** (up from 20)

## New Configuration Options

| Key | Default | Description |
|-----|---------|-------------|
| `review.rules_file` | `.merlin-rules.yaml` | Path to the custom rules file |
| `review.feedback_learning` | `false` | Enable adaptive feedback filtering |
| `review.feedback_path` | `.merlin-feedback.jsonl` | Path to the feedback data file |

## New Modules

| Module | Description |
|--------|-------------|
| `feedback` | Adaptive feedback store — records accept/reject signals, filters noisy patterns |
| `rules` | Custom rules engine — loads `.merlin-rules.yaml`, compiles regex patterns, generates AI prompt directives |
| `tools/diagram` | `/diagram` slash command implementation |
| `tools/feedback` | `/feedback` slash command implementation |

## Dependencies

- Added `serde_yaml_ng` 0.10 for YAML parsing of `.merlin-rules.yaml`

## Compatibility

- Rust 1.75+ (unchanged)
- All 12 AI providers supported (unchanged)
- All 5 VCS platforms supported (unchanged)
- Fully backwards compatible — new features are opt-in via config

## Full Changelog

- feat: add custom review rules engine with regex patterns, directives, and path scoping
- feat: add adaptive feedback learning to suppress noisy comment patterns
- feat: add `/diagram` command for PR architecture diagrams in Mermaid
- feat: add `/feedback` command for feedback learning status
- feat: inject custom rule matches as AI context hints during review
- feat: inject rule directives into AI system prompt
- feat: add feedback-based comment filtering to ReviewEngine pipeline
- docs: update README with new features, config options, and architecture
- chore: bump version to 0.2.0
