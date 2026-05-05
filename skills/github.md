---
description: "GitHub operations via the gh CLI: repos, PRs, issues, commits, and conventional commits"
version: 1.0.0
tags: [development, github, git, vcs]
requires_tools: [bash, api_keys_check]
---

# GitHub Operations

## Prerequisites

The `gh` CLI must be installed and authenticated. Check with:
```bash
gh auth status
```

For API token operations, check `GITHUB_TOKEN`:
```
api_keys_check(service_name: "GITHUB_TOKEN")
```

## Repository Operations

### Clone
```bash
gh repo clone owner/repo
```

### Create
```bash
gh repo create repo-name --public --description "Description"
```

### View
```bash
gh repo view owner/repo
```

## Pull Requests

### List PRs
```bash
gh pr list --state open
```

### Create PR
```bash
gh pr create --title "feat: add feature" --body "Description" --base main
```

### View PR
```bash
gh pr view 123
```

### Merge PR
```bash
gh pr merge 123 --squash --delete-branch
```

## Issues

### List Issues
```bash
gh issue list --state open --label bug
```

### Create Issue
```bash
gh issue create --title "Bug: description" --body "Details" --label bug
```

### Close Issue
```bash
gh issue close 123 --comment "Fixed in #456"
```

## Commits (Conventional Commits)

Follow the conventional commits format:
- `feat:` — New feature
- `fix:` — Bug fix
- `docs:` — Documentation
- `chore:` — Maintenance
- `refactor:` — Code refactoring
- `test:` — Adding tests
- `ci:` — CI changes

Example:
```bash
git commit -m "feat(auth): add OAuth2 login support"
```

## Workflow

### Feature Branch Flow
1. `git checkout -b feat/feature-name`
2. Make changes, commit with conventional commits
3. `gh pr create --title "feat: description" --base main`
4. After review: `gh pr merge --squash --delete-branch`

### Release Flow
1. Check recent changes: `gh pr list --state merged --limit 20`
2. Create release: `gh release create v1.0.0 --generate-notes`
