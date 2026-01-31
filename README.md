# dit — Docker Image Tracker 🐋

**Track Docker image sizes over time. Know exactly which commit made your image fat.**

A fast, single-binary CLI tool that monitors Docker image sizes across commits with layer-by-layer analysis, historical tracking, and CI integration that comments on your PRs automatically.

## Why?

Your Docker images keep growing and nobody notices until deployment takes forever. `dit` catches size regressions early — in CI, before they hit production.

## Features

- 📊 **Track** image sizes tied to git commits
- 🔍 **Analyze** layer-by-layer breakdowns
- 🔄 **Diff** any two snapshots to see what changed
- 📈 **History** with trend indicators
- 🐳 **Multi-image** — track all images at once with `--filter`
- 🚀 **CI mode** — auto-comments on PRs with size changes
- ⚡ **Fast** — single Rust binary, no dependencies

## Quick Start

```bash
# Install
git clone https://github.com/Bentlybro/docker-image-tracker
cd docker-image-tracker
cargo install --path .

# Analyze an image
dit analyze myapp:latest

# Track it (saves snapshot with git context)
dit track myapp:latest

# Track all matching images at once
dit track-all --filter myapp

# After changes, see what grew
dit diff myapp:latest

# View history
dit history myapp:latest
```

## Commands

| Command | Description |
|---------|-------------|
| `dit analyze <image>` | Inspect image with layer breakdown |
| `dit analyze-all` | Analyze all local images |
| `dit track <image>` | Record snapshot with git context |
| `dit track-all` | Track all images (with `--filter`) |
| `dit diff <image>` | Compare snapshots |
| `dit history <image>` | View size timeline |
| `dit compose analyze\|track\|history` | Docker Compose support |
| `dit summary` | Dashboard of all tracked images |
| `dit ci` | CI mode with PR comments |

### Analyze

```bash
$ dit analyze myapp:latest

 Image: myapp:latest
 Size: 245.3 MB | Layers: 12 | OS: linux/amd64

 # │   Size   │  Created   │ Command
 1 │ 80.4 MB  │ 2026-01-15 │ FROM node:18-alpine
 2 │ 45.2 MB  │ 2026-01-30 │ RUN npm install
 3 │ 18.7 MB  │ 2026-01-30 │ RUN npm run build
 ...
```

### Track All

```bash
$ dit track-all --filter autogpt_platform
Tracking 8 images at commit 7ee94d9...

  autogpt_platform-frontend:latest      ... ✅ 125.5 MiB
  autogpt_platform-executor:latest      ... ✅ 508.4 MiB
  autogpt_platform-rest_server:latest   ... ✅ 508.4 MiB
  ...

✅ Tracked 8 images, total size: 3.6 GiB
```

### Diff

```bash
$ dit diff myapp:latest

 Before (abc123): 245.3 MB
 After  (def456): 267.8 MB
 Change: +22.5 MB (+9.2%) 📈

 Status   │   Size   │   Delta   │ Command
 Modified │ 63.7 MB  │ +18.5 MB  │ RUN npm install
 Added    │  3.8 MB  │  +3.8 MB  │ COPY ./dist
 Same     │ 89.1 MB  │     —     │ FROM node:18-alpine
```

### Summary Dashboard

```bash
$ dit summary

 Image                        │ Size     │ Trend         │ Last Tracked
 autogpt_platform-frontend    │ 125.5 MB │ +5.2 MB → →   │ 2026-01-31
 autogpt_platform-executor    │ 508.4 MB │ → → -8 MB     │ 2026-01-31
 autogpt_platform-rest_server │ 508.4 MB │ →             │ 2026-01-31

 Total: 3.6 GiB across 8 images
```

## CI Integration

### GitHub Action

```yaml
name: Docker Size Check
on: [pull_request]

jobs:
  size-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: docker compose build
      - uses: Bentlybro/docker-image-tracker@v1
        with:
          filter: autogpt_platform
          budget: 5GB
          comment: true
```

Every PR gets an automatic comment:

> ## 🐋 Docker Image Size Report
>
> | Image | Previous | Current | Change |
> |-------|----------|---------|--------|
> | frontend | 120.3 MB | 125.5 MB | +5.2 MB (+4.3%) 📈 |
> | executor | 508.4 MB | 508.4 MB | — ✅ |
> | **Total** | **3.4 GB** | **3.6 GB** | **+200 MB (+5.9%)** |
>
> <details><summary>Layer details for frontend</summary>
>
> | Status | Size | Delta | Command |
> |--------|------|-------|---------|
> | Modified 🔄 | 63.7 MB | +18.5 MB | `RUN npm install` |
> | Added ➕ | 3.8 MB | +3.8 MB | `COPY ./dist` |
>
> </details>
>
> ✅ Budget: 3.6 GB / 5 GB

### `dit ci` Command

```bash
# Single image with budget
dit ci myapp:latest --budget 500MB --github-comment

# Multiple images
dit ci --filter autogpt_platform --budget 5GB --github-comment

# Strict mode — fail on any increase
dit ci --filter production --fail-on-increase --github-comment

# Compare against main branch
dit ci myapp:latest --base main --github-comment
```

**Flags:**
- `--budget <size>` — Max total size (e.g., `500MB`, `5GB`). Fails CI if exceeded
- `--budget-increase <percent>` — Max increase % per image
- `--fail-on-increase` — Fail if any image grew at all
- `--github-comment` — Post/update PR comment (needs `GITHUB_TOKEN`)
- `--base <branch>` — Compare against specific branch
- `--filter <pattern>` — Filter by image name
- `--format json|markdown|table` — Output format

### Action Inputs

| Input | Description | Default |
|-------|-------------|---------|
| `image` | Image to track | — |
| `filter` | Filter by name | — |
| `budget` | Max size (e.g., `5GB`) | — |
| `budget-increase` | Max increase % | — |
| `comment` | Post PR comment | `true` |
| `fail-on-increase` | Fail on growth | `false` |
| `base` | Baseline branch | latest |
| `token` | GitHub token | `github.token` |

## How It Works

1. **Docker API** — Inspects images via the Docker daemon ([bollard](https://github.com/fussybeaver/bollard))
2. **Git context** — Captures commit SHA, branch, author, message
3. **Local storage** — Saves to `.dit/history.json`
4. **Layer diffing** — Compares digests to detect changes
5. **PR comments** — Updates existing comment (no spam) via GitHub API

## Install

**From source** (requires [Rust](https://rustup.rs/) 1.70+):
```bash
cargo install --path .
```

**Pre-built binaries** — coming soon with GitHub Releases.

## Roadmap

- [x] Core CLI (analyze, track, diff, history)
- [x] Multi-image support (analyze-all, track-all, compose, summary)
- [x] CI integration (dit ci, GitHub Action, PR comments)
- [ ] Pre-built release binaries
- [ ] ASCII trend charts
- [ ] HTML reports
- [ ] Registry support (analyze without pulling)
- [ ] GitLab CI / other CI platforms

## License

MIT — see [LICENSE](LICENSE)

---

Built by [Bentlybro](https://github.com/Bentlybro) with [Orion](https://github.com/openclaw/openclaw) ⭐
