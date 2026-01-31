# Docker Image Tracker — Design Document

## Vision
A CLI tool + CI integration that tracks Docker image sizes over time, pinpointing exactly which commit/change caused size increases. Think "bundle size tracking for Docker images."

## Core Features

### 1. Image Analysis (`dit analyze`)
- Inspect a Docker image and extract detailed metrics:
  - Total image size (compressed + uncompressed)
  - Layer-by-layer breakdown (size, command, created timestamp)
  - Base image identification
  - Number of layers
- Output as JSON, table, or markdown

### 2. History Tracking (`dit track`)
- Record image metrics to a local JSON/SQLite database
- Associate each record with: git commit SHA, branch, timestamp, tag
- Compare any two snapshots: `dit diff <commit-a> <commit-b>`
- Show timeline: `dit history --last 20`

### 3. CI Integration (`dit ci`)
- Run in CI (GitHub Actions, GitLab CI, etc.)
- Compare current build against the base branch
- Post a comment on the PR with:
  - Size change (absolute + percentage)
  - Layer-by-layer diff (which layers grew/shrank/added/removed)
  - Largest layers highlighted
  - Badge showing trend (📈 grew, 📉 shrank, ✅ unchanged)
- Configurable size budgets — fail the build if image exceeds threshold
- Store history as a JSON artifact or in a branch (`dit-data`)

### 4. Reporting (`dit report`)
- Generate markdown/HTML report of size trends over time
- Visualize with ASCII charts in terminal
- Export data for external dashboards

## Tech Stack

### Rust 🦀
- **Why**: Single binary, no runtime dependencies, cross-platform, fast
- **Docker interaction**: Use `bollard` crate (Docker Engine API client) or shell out to `docker inspect`
- **CLI framework**: `clap` (industry standard for Rust CLIs)
- **JSON**: `serde` + `serde_json`
- **HTTP** (for GitHub API comments): `reqwest`
- **Tables/formatting**: `comfy-table` or `tabled`
- **Storage**: `serde_json` to flat files (keep it simple), or `rusqlite` if we need queries

### GitHub Action
- Pre-built binary downloaded in action setup step
- Action YAML wrapper that calls `dit ci`
- Marketplace-ready from day one

## Architecture

```
docker-image-tracker/
├── src/
│   ├── main.rs              # CLI entry point (clap)
│   ├── analyze.rs           # Image inspection & metrics extraction
│   ├── track.rs             # History recording & storage
│   ├── diff.rs              # Comparison logic
│   ├── ci.rs                # CI mode (PR comments, badge generation)
│   ├── report.rs            # Report generation
│   ├── docker.rs            # Docker API interaction layer
│   └── models.rs            # Data structures
├── action/
│   └── action.yml           # GitHub Action definition
├── Cargo.toml
├── README.md
├── LICENSE (MIT)
└── DESIGN.md
```

## Data Model

```rust
struct ImageSnapshot {
    // Identity
    image: String,           // e.g., "myapp:latest"
    tag: Option<String>,
    digest: Option<String>,
    
    // Git context
    commit_sha: String,
    branch: String,
    commit_message: String,
    author: String,
    timestamp: DateTime<Utc>,
    
    // Metrics
    total_size: u64,         // bytes (uncompressed)
    compressed_size: Option<u64>,
    layer_count: usize,
    layers: Vec<LayerInfo>,
    
    // Metadata
    base_image: Option<String>,
    os: String,
    arch: String,
}

struct LayerInfo {
    digest: String,
    size: u64,
    command: String,         // Dockerfile instruction that created this
    created: DateTime<Utc>,
}

struct SizeDiff {
    before: ImageSnapshot,
    after: ImageSnapshot,
    total_delta: i64,        // positive = grew
    layer_changes: Vec<LayerChange>,
}

enum LayerChange {
    Added(LayerInfo),
    Removed(LayerInfo),
    Modified { before: LayerInfo, after: LayerInfo },
    Unchanged(LayerInfo),
}
```

## CLI Interface

```bash
# Analyze current image
dit analyze myapp:latest
dit analyze myapp:latest --format json

# Track (record a snapshot)
dit track myapp:latest                    # auto-detects git context
dit track myapp:latest --commit abc123    # manual override

# Compare
dit diff myapp:latest --base main         # compare current vs main branch
dit diff myapp:latest abc123 def456       # compare two specific commits

# History
dit history myapp:latest --last 20
dit history myapp:latest --since "2 weeks ago"

# CI mode (all-in-one for pipelines)
dit ci myapp:latest --budget 500MB --github-comment --fail-over 10%

# Report
dit report myapp:latest --format markdown
dit report myapp:latest --format html --output report.html
```

## CI Workflow Example (GitHub Actions)

```yaml
name: Docker Image Size Check
on: [pull_request]

jobs:
  size-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: Bentlybro/docker-image-tracker@v1
        with:
          image: myapp:latest
          budget: 500MB
          fail-over-percent: 10
          comment: true
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

## PR Comment Example

```markdown
## 🐋 Docker Image Size Report

| Metric | Base (`main`) | This PR | Change |
|--------|--------------|---------|--------|
| Total Size | 245.3 MB | 267.8 MB | +22.5 MB (+9.2%) 📈 |
| Layers | 12 | 14 | +2 |
| Compressed | 98.1 MB | 112.4 MB | +14.3 MB |

### Layer Changes
| # | Command | Before | After | Delta |
|---|---------|--------|-------|-------|
| 8 | `RUN npm install` | 45.2 MB | 63.7 MB | **+18.5 MB** ⚠️ |
| 12 | `COPY ./dist` | - | 3.8 MB | +3.8 MB (new) |
| 6 | `RUN apt-get install` | 89.1 MB | 89.1 MB | unchanged |

💡 **Tip**: The `npm install` layer grew significantly. Consider using `npm ci --production` or a multi-stage build.

📊 Budget: 500 MB — ✅ Within budget
```

## Storage Strategy

### Option A: JSON files in repo (simplest)
- Store snapshots in `.dit/history.json`
- Committed to repo (or a dedicated branch)
- Works everywhere, no external deps

### Option B: GitHub Action artifacts
- Store as build artifacts
- Download previous artifact for comparison
- No repo pollution

### Option C: Dedicated `dit-data` branch
- Orphan branch storing only tracking data
- Clean separation from source code
- Works well for CI

**Recommendation**: Start with Option A for local use, Option B/C for CI. Make storage pluggable.

## MVP Scope (v0.1.0)

Phase 1 — Core CLI:
- [x] `dit analyze` — inspect image, show layer breakdown
- [x] `dit track` — record snapshot to local JSON
- [x] `dit diff` — compare two snapshots
- [x] `dit history` — show timeline

Phase 2 — CI Integration:
- [ ] `dit ci` — all-in-one CI command
- [ ] GitHub PR comment posting
- [ ] Size budget enforcement
- [ ] GitHub Action wrapper

Phase 3 — Polish:
- [ ] ASCII chart trends in terminal
- [ ] HTML report generation
- [ ] Multiple image tracking
- [ ] Docker Hub / registry support (analyze without pulling)

## Name Ideas
- `dit` (Docker Image Tracker) — short, memorable, easy to type ✅
- `docker-diet` — fun but longer
- `docksize` — decent
- `blubber` — because whale fat 🐋

**Going with `dit` as the CLI command name.**
