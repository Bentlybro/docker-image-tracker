# dit — Docker Image Tracker 🐋

**Track Docker image sizes over time and catch bloat before it reaches production.**

`dit` is a CLI tool that monitors Docker image sizes across commits, providing detailed layer-by-layer analysis and historical tracking. Perfect for keeping container images lean and identifying size regressions in CI/CD pipelines.

## Features

- 🔍 **Analyze** — Inspect any Docker image and see a detailed layer breakdown
- 🔍 **Analyze All** — Analyze ALL local images at once (with filtering)
- 📊 **Track** — Record image snapshots with git context (commit, branch, author)
- 📊 **Track All** — Track multiple images in one command
- 🐳 **Compose Support** — Track all images from docker-compose.yml
- 📋 **Summary** — Dashboard view of all tracked images
- 🔄 **Diff** — Compare any two snapshots and see exactly what changed
- 📈 **History** — View size trends across commits with delta indicators
- 🎨 **Beautiful output** — Color-coded tables with human-readable sizes
- 🚀 **Fast** — Written in Rust, single binary, no runtime dependencies
- 🔌 **CI-ready** — JSON output for automation (GitHub Actions coming soon)

## Installation

### From Source

Requires [Rust](https://rustup.rs/) 1.70+:

```bash
git clone https://github.com/Bentlybro/docker-image-tracker
cd docker-image-tracker
cargo install --path .
```

The binary will be installed to `~/.cargo/bin/dit`.

### Pre-built Binaries

Coming soon! 🚧

## Quick Start

```bash
# Build or pull a Docker image
docker build -t myapp:latest .

# Analyze the image
dit analyze myapp:latest

# Track it (saves a snapshot with git context)
dit track myapp:latest

# Make some changes, rebuild, and track again
# ... make changes to Dockerfile ...
docker build -t myapp:latest .
dit track myapp:latest

# See the size change
dit diff myapp:latest

# View full history
dit history myapp:latest --last 10
```

## Commands

### `dit analyze <image>`

Inspect a Docker image and display detailed layer information.

```bash
$ dit analyze myapp:latest

Image Analysis
Image: myapp
Tag: latest
Total Size: 245.3 MB
Layers: 12
OS/Arch: linux/amd64

Layer Breakdown
╭───┬───────────┬────────────┬─────────────────────────────────────────────╮
│ # │   Size    │  Created   │                  Command                    │
├───┼───────────┼────────────┼─────────────────────────────────────────────┤
│ 1 │  20.4 MB  │ 2024-01-15 │ <layer>                                     │
│ 2 │  20.4 MB  │ 2024-01-15 │ <layer>                                     │
│...│    ...    │    ...     │                  ...                        │
╰───┴───────────┴────────────┴─────────────────────────────────────────────╯
```

**Options:**
- `--format json` — Output raw JSON for scripting

### `dit track <image>`

Record a snapshot of the image with git context.

```bash
$ dit track myapp:latest
✅ Tracked snapshot for myapp at commit a1b2c3d
Branch: feature/optimize-build
Size: 257345678 bytes
```

Snapshots are saved to `.dit/history.json` in your project directory.

### `dit diff <image> [commit-a] [commit-b]`

Compare two image snapshots.

```bash
# Compare last two snapshots
$ dit diff myapp:latest

# Compare specific commits
$ dit diff myapp:latest abc123 def456

# Compare against a branch
$ dit diff myapp:latest --base main

Image Size Diff
Image: myapp
Before (abc123): 245.3 MB
After (def456): 267.8 MB
Change: +22.5 MB (+9.2%) 📈

Layer Changes
╭──────────┬───────────┬───────────┬─────────────────────────────────────────╮
│  Status  │   Size    │   Delta   │                Command                  │
├──────────┼───────────┼───────────┼─────────────────────────────────────────┤
│ Modified │  63.7 MB  │ +18.5 MB  │ <layer>                                 │
│ Added    │   3.8 MB  │  +3.8 MB  │ <layer>                                 │
│ Unchanged│  89.1 MB  │ unchanged │ <layer>                                 │
╰──────────┴───────────┴───────────┴─────────────────────────────────────────╯
```

**Options:**
- `--base <branch>` — Compare against the latest snapshot from a specific branch

### `dit history <image>`

View historical size data for an image.

```bash
$ dit history myapp:latest --last 5

Image Size History
Image: myapp
╭─────────┬─────────┬──────────────────┬──────────┬───────────┬───────╮
│ Commit  │ Branch  │       Date       │   Size   │   Delta   │ Trend │
├─────────┼─────────┼──────────────────┼──────────┼───────────┼───────┤
│ a1b2c3d │ main    │ 2024-01-15 10:30 │ 245.3 MB │     —     │  —    │
│ d4e5f6g │ main    │ 2024-01-16 14:22 │ 267.8 MB │ +22.5 MB  │  📈   │
│ h7i8j9k │ feature │ 2024-01-17 09:15 │ 251.2 MB │ -16.6 MB  │  📉   │
│ l0m1n2o │ feature │ 2024-01-17 11:45 │ 251.2 MB │ unchanged │  ✅   │
╰─────────┴─────────┴──────────────────┴──────────┴───────────┴───────╯
```

**Options:**
- `--last N` — Show only the last N snapshots

### `dit analyze-all [--filter <pattern>]`

Analyze ALL local Docker images at once. Perfect for getting an overview of your entire Docker environment.

```bash
$ dit analyze-all

All Docker Images
╭──────────────────────┬─────────┬───────────┬────────┬─────────────╮
│        Image         │   Tag   │   Size    │ Layers │   OS/Arch   │
├──────────────────────┼─────────┼───────────┼────────┼─────────────┤
│ autogpt_platform-db  │ latest  │  2.25 GB  │   18   │ linux/amd64 │
│ autogpt_platform-api │ latest  │  2.25 GB  │   18   │ linux/amd64 │
│ autogpt_platform-ui  │ latest  │  2.25 GB  │   18   │ linux/amd64 │
│ postgres             │ 13      │  314.2 MB │   12   │ linux/amd64 │
│ redis                │ alpine  │   28.5 MB │    7   │ linux/amd64 │
╰──────────────────────┴─────────┴───────────┴────────┴─────────────╯

Total: 5 images, 9.4 GB combined
```

**Filter by name:**
```bash
# Only show images matching "autogpt"
$ dit analyze-all --filter autogpt_platform

All Docker Images
╭──────────────────────┬─────────┬───────────┬────────┬─────────────╮
│        Image         │   Tag   │   Size    │ Layers │   OS/Arch   │
├──────────────────────┼─────────┼───────────┼────────┼─────────────┤
│ autogpt_platform-db  │ latest  │  2.25 GB  │   18   │ linux/amd64 │
│ autogpt_platform-api │ latest  │  2.25 GB  │   18   │ linux/amd64 │
│ autogpt_platform-ui  │ latest  │  2.25 GB  │   18   │ linux/amd64 │
╰──────────────────────┴─────────┴───────────┴────────┴─────────────╯

Total: 3 images, 6.75 GB combined
```

**Options:**
- `--filter <pattern>` — Only show images matching the pattern (case-insensitive substring match)
- `--format json` — Output as JSON for scripting

### `dit track-all [--filter <pattern>]`

Track all local images in one command. Captures git context once and applies it to all snapshots.

```bash
$ dit track-all --filter autogpt_platform
Tracking 8 images at commit a1b2c3d...

  autogpt_platform-db:latest ... ✅ 2.25 GB tracked
  autogpt_platform-api:latest ... ✅ 2.25 GB tracked
  autogpt_platform-ui:latest ... ✅ 2.25 GB tracked
  autogpt_platform-worker-1:latest ... ✅ 2.25 GB tracked
  autogpt_platform-worker-2:latest ... ✅ 2.25 GB tracked
  autogpt_platform-worker-3:latest ... ✅ 2.25 GB tracked
  autogpt_platform-worker-4:latest ... ✅ 2.25 GB tracked
  autogpt_platform-scheduler:latest ... ✅ 2.25 GB tracked

✅ Tracked 8 images, total size: 18.0 GB
```

**Options:**
- `--filter <pattern>` — Only track images matching the pattern

### `dit compose <subcommand>`

Work with Docker Compose projects. Automatically detects services with `build:` directives.

#### `dit compose analyze [--file <path>]`

Analyze all images built by docker-compose.

```bash
$ dit compose analyze
Found 8 services with build directives in ./docker-compose.yml:

Analyzing 8 compose images...

  autogpt_platform-db:latest — 2.25 GB (18 layers)
  autogpt_platform-api:latest — 2.25 GB (18 layers)
  autogpt_platform-ui:latest — 2.25 GB (18 layers)
  autogpt_platform-worker-1:latest — 2.25 GB (18 layers)
  ...
```

#### `dit compose track [--file <path>]`

Track all compose-built images.

```bash
$ dit compose track
Tracking 8 compose images...
  autogpt_platform-db:latest ... ✅ 2.25 GB tracked
  autogpt_platform-api:latest ... ✅ 2.25 GB tracked
  ...
```

#### `dit compose history [--file <path>]`

Show history for all compose services.

**Options (all subcommands):**
- `--file <path>` — Use a specific compose file (defaults to auto-detect in current directory)

### `dit summary`

Show a dashboard-style overview of all tracked images.

```bash
$ dit summary

Docker Image Tracker Summary
Total tracked images: 8

╭──────────────────────────┬─────────────┬──────────────────────┬───────────┬──────────────────╮
│          Image           │ Latest Size │   Trend (Last 3)     │ Snapshots │   Last Tracked   │
├──────────────────────────┼─────────────┼──────────────────────┼───────────┼──────────────────┤
│ autogpt_platform-db      │   2.25 GB   │ +45 MB → +12 MB      │     5     │ 2024-01-17 14:30 │
│ autogpt_platform-api     │   2.25 GB   │ → → -8 MB            │     3     │ 2024-01-17 14:30 │
│ autogpt_platform-ui      │   2.25 GB   │ +120 MB → →          │     4     │ 2024-01-17 14:30 │
│ postgres:13              │  314.2 MB   │ —                    │     1     │ 2024-01-15 09:00 │
╰──────────────────────────┴─────────────┴──────────────────────┴───────────┴──────────────────╯

Total combined size: 9.1 GB
```

The trend shows deltas between the last 2-3 snapshots:
- `→` = no change
- `+X MB` = increased (red)
- `-X MB` = decreased (green)

## CI Integration (Coming Soon)

Imagine this comment on your PRs:

> ## 🐋 Docker Image Size Report
> 
> | Metric | Base (`main`) | This PR | Change |
> |--------|--------------|---------|--------|
> | Total Size | 245.3 MB | 267.8 MB | +22.5 MB (+9.2%) 📈 |
> | Layers | 12 | 14 | +2 |
> 
> ### Layer Changes
> | Command | Before | After | Delta |
> |---------|--------|-------|-------|
> | `RUN npm install` | 45.2 MB | 63.7 MB | **+18.5 MB** ⚠️ |
> | `COPY ./dist` | - | 3.8 MB | +3.8 MB (new) |
> 
> 💡 **Tip**: The `npm install` layer grew significantly. Consider using `npm ci --production` or a multi-stage build.
> 
> 📊 Budget: 500 MB — ✅ Within budget

GitHub Action coming in Phase 2!

## How It Works

1. **Docker API** — Uses [bollard](https://github.com/fussybeaver/bollard) to inspect images via the Docker daemon
2. **Git Integration** — Shells out to `git` to capture commit context (SHA, branch, author, message)
3. **Local Storage** — Saves snapshots to `.dit/history.json` (JSON array)
4. **Diffing** — Compares layer digests to detect additions, removals, and modifications

## Requirements

- **Docker** — Must be running locally
- **Git** — Required for `dit track` (auto-detects commit info)
- **Rust** — 1.70+ (for building from source)

## Roadmap

- [x] Phase 1: Core CLI (`analyze`, `track`, `diff`, `history`)
- [x] Phase 1.5: Multi-image support (`analyze-all`, `track-all`, `compose`, `summary`)
- [ ] Phase 2: CI integration (GitHub Actions, GitLab CI)
- [ ] Phase 3: Advanced features (charts, HTML reports, registry support)

## Contributing

Contributions welcome! This is a brand new project (v0.1.0).

## License

MIT License - see [LICENSE](LICENSE) for details.

## Author

Built by [Bentlybro](https://github.com/Bentlybro) 🦀

---

**Tip**: Add `.dit/` to your `.gitignore` if you don't want to commit history to your repo.
