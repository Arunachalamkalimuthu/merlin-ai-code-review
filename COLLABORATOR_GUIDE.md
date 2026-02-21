# Collaborator Guide

This guide is for project collaborators — people with commit or merge access to the Merlin repository. It covers release management, triage workflows, PR review standards, and maintainer responsibilities.

---

## Roles

| Role | Permissions | Responsibilities |
|---|---|---|
| **Contributor** | Fork + PR | Submit code, report issues |
| **Collaborator** | Push to branches, merge PRs | Review PRs, triage issues, cut releases |
| **Maintainer** | Admin access | Roadmap, architecture decisions, final release authority |

---

## Becoming a Collaborator

Contributors who have made several quality contributions (bug fixes, features, or significant docs) may be invited to become collaborators. Collaborators are expected to:

- Review PRs within a reasonable timeframe (aim for 2–3 business days)
- Follow the review standards in this guide
- Be available to discuss architecture decisions in issues/discussions
- Uphold the [Code of Conduct](CODE_OF_CONDUCT.md)

---

## PR Review Standards

### Who can merge

- Any collaborator can merge PRs from external contributors after approval
- Collaborators should not merge their own PRs — request a review from another collaborator
- Trivial fixes (typo, doc-only changes) may be self-merged with a comment explaining why

### Required checks before merging

- [ ] CI passes (`cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`)
- [ ] At least one approving review
- [ ] No unresolved conversations
- [ ] Commit messages follow [conventional commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `docs:`, `chore:`, `perf:`, `refactor:`, `test:`)
- [ ] `CHANGELOG.md` updated for user-facing changes
- [ ] Documentation updated if behaviour changes

### What to look for in reviews

**Correctness**
- Does it do what the PR description says?
- Are edge cases handled?
- Are error paths covered?

**Security**
- Are API keys or secrets ever logged or exposed?
- Is user input validated at boundaries?
- Are new HTTP endpoints authenticated?

**Performance**
- Are there unnecessary clones or allocations in hot paths?
- Does the change affect CI runtime?

**Tests**
- Is there a test for new logic?
- Do existing tests still pass and make sense?

**Docs**
- If a config key is added, is it documented in `merlin.toml` reference?
- If a CLI flag is added, is it in the README CLI reference?

---

## Issue Triage

When a new issue is opened, a collaborator should triage it within 48 hours.

### Labels to apply

| Label | When |
|---|---|
| `bug` | Confirmed bug report |
| `enhancement` | New feature request |
| `documentation` | Docs-only issue |
| `question` | Usage question (close after answering) |
| `good first issue` | Small, well-defined, appropriate for new contributors |
| `help wanted` | Needs contributors, non-trivial |
| `security` | Security issue — handle privately first |
| `wontfix` | Out of scope or won't be addressed |
| `duplicate` | Duplicate of another issue |
| `needs repro` | Bug needs a reproduction case |

### Triage checklist

- [ ] Is it a security issue? → close, redirect to private reporting (see [SECURITY.md](SECURITY.md))
- [ ] Is it a duplicate? → close with reference to original
- [ ] Is it clearly reproducible? → add `needs repro` if not
- [ ] Apply appropriate labels
- [ ] Assign milestone if relevant
- [ ] Thank the reporter

---

## Release Process

### Version numbering

Merlin follows [Semantic Versioning](https://semver.org/):

- `MAJOR.MINOR.PATCH`
- `PATCH` — backwards-compatible bug fixes
- `MINOR` — new backwards-compatible features
- `MAJOR` — breaking changes (config format, CLI flags, API)

Pre-releases use `-rc.N` suffix: `v0.2.0-rc.1`

### Cutting a release

1. **Update `CHANGELOG.md`**

   Move the `## [Unreleased]` section to a versioned heading:

   ```markdown
   ## [0.2.0] — 2026-03-01
   ```

2. **Bump version in `Cargo.toml`**

   ```toml
   [package]
   version = "0.2.0"
   ```

3. **Commit**

   ```bash
   git add Cargo.toml CHANGELOG.md
   git commit -m "chore: release v0.2.0"
   git push
   ```

4. **Tag and push**

   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

   The [Release workflow](.github/workflows/release.yml) triggers automatically and:
   - Builds 7 platform binaries (Linux amd64/arm64 glibc+musl, macOS amd64/arm64, Windows amd64)
   - Creates a GitHub release with auto-generated changelog
   - Pushes Docker images to GHCR (`latest`, `0.2.0`, `0.2`, `0`)

5. **Verify the release**

   - Check the [Actions tab](https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/actions) — all jobs should go green
   - Check the [Releases page](https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases) — all 7 binaries + installers should be attached
   - Check [GHCR](https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/pkgs/container/merlin-ai-code-review) — new image tags should be listed

### Hotfix releases

For critical bugs on a released version:

```bash
git checkout -b hotfix/v0.1.1 v0.1.0
# make fix
git commit -m "fix: critical bug description"
git tag v0.1.1
git push origin hotfix/v0.1.1 v0.1.1
# open PR from hotfix branch → main
```

---

## Branch Strategy

| Branch | Purpose |
|---|---|
| `main` | Always releasable. PRs merge here. |
| `feat/*` | Feature branches — short-lived |
| `fix/*` | Bug fix branches |
| `hotfix/*` | Emergency fixes branched from a tag |
| `docs/*` | Documentation-only changes |
| `chore/*` | Maintenance, dependency updates |

Direct pushes to `main` are reserved for collaborators making trivial, low-risk changes (e.g. a one-line typo fix). Anything non-trivial goes through a PR.

---

## Dependency Updates

Dependabot opens automated PRs for dependency updates. Collaborator responsibilities:

- **Patch updates** — merge if CI is green, no review needed
- **Minor updates** — review changelog for breaking changes, merge if clean
- **Major updates** — requires a collaborator to review and test manually
- **Security updates** — prioritise and merge promptly

---

## Communication

| Channel | Purpose |
|---|---|
| GitHub Issues | Bug reports, feature requests |
| GitHub Discussions | Architecture questions, RFCs, community Q&A |
| GitHub PRs | Code review |
| Security advisories | Private vulnerability reports |

Keep technical discussions in GitHub so they are searchable and public. Avoid making architecture decisions in private channels.

---

## Code of Conduct Enforcement

Collaborators are responsible for enforcing the [Code of Conduct](CODE_OF_CONDUCT.md). When a violation is reported:

1. Acknowledge the report privately within 24 hours
2. Discuss with at least one other collaborator before acting
3. Take the appropriate enforcement action (see Code of Conduct enforcement guidelines)
4. Document the decision privately
5. Follow up with the reporter

When in doubt, err on the side of the reporter's safety over the accused's comfort.
