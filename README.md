# Smelt

Containerized job execution engine for agentic development workflows.

Smelt is the infrastructure layer in the **Smelt / Assay / Cupel** toolkit. It provisions isolated Docker (or Compose, or Kubernetes) environments, mounts the host repo, delegates orchestration to Assay inside the container, streams gate output to the terminal, collects the result branch, and creates a GitHub PR for human review.

**The output of `smelt run` is a pull request, ready for review.**

---

## Install

Build and install from source (requires [Rust](https://rustup.rs/)):

```bash
cargo install --path .
```

Or build without installing:

```bash
cargo build --release
# binary at target/release/smelt
```

---

## Quickstart

```bash
# 1. Generate a skeleton manifest in the current directory
smelt init

# 2. Edit the generated job-manifest.toml — set the image, sessions, and credentials

# 3. Validate the manifest and preview the execution plan
smelt run job-manifest.toml --dry-run

# 4. Run the job (provisions container, runs Assay, collects results)
smelt run job-manifest.toml
```

See [`examples/`](examples/) for complete manifest samples covering Docker, Compose, and Kubernetes runtimes.

---

## Subcommands

### `smelt init`

Generate a skeleton job manifest in the current directory.

```
Usage: smelt init

Options:
  -h, --help     Print help
  -V, --version  Print version
```

**Example:**

```bash
smelt init
# Creates job-manifest.toml with a commented template
```

---

### `smelt list`

List past runs recorded in `.smelt/runs/`.

```
Usage: smelt list [OPTIONS]

Options:
      --dir <DIR>  Directory to search for `.smelt/runs/` (defaults to current directory) [default: .]
  -h, --help       Print help
  -V, --version    Print version
```

| Flag | Description |
|------|-------------|
| `--dir <DIR>` | Directory to search for `.smelt/runs/` (defaults to `.`) |

**Example:**

```bash
smelt list
smelt list --dir /path/to/project
```

---

### `smelt run`

Run a job manifest — provisions the container environment, executes Assay sessions, collects results, and optionally creates a PR.

```
Usage: smelt run [OPTIONS] <MANIFEST>

Arguments:
  <MANIFEST>  Path to the job manifest TOML file

Options:
      --dry-run  Validate and print the execution plan without running anything
      --no-pr    Skip PR creation even when a `[forge]` section is present in the manifest
  -h, --help     Print help
  -V, --version  Print version
```

| Flag | Description |
|------|-------------|
| `--dry-run` | Validate and print the execution plan without running anything |
| `--no-pr` | Skip PR creation even when a `[forge]` section is present in the manifest |

**Examples:**

```bash
# Dry run — validate and preview
smelt run examples/job-manifest.toml --dry-run

# Full run
smelt run job-manifest.toml

# Run without creating a PR
smelt run job-manifest.toml --no-pr
```

---

### `smelt serve`

Start the job dispatch daemon. Watches a directory for incoming manifest files, accepts jobs via HTTP API, dispatches up to `max_concurrent` parallel jobs, auto-retries failures, and displays a live TUI dashboard.

```
Usage: smelt serve [OPTIONS] --config <CONFIG>

Options:
  -c, --config <CONFIG>  Path to the server configuration TOML file
      --no-tui           Disable the Ratatui TUI (tracing output stays on stderr)
  -h, --help             Print help
  -V, --version          Print version
```

| Flag | Description |
|------|-------------|
| `-c, --config <CONFIG>` | **(Required)** Path to the server configuration TOML file |
| `--no-tui` | Disable the Ratatui TUI (tracing output stays on stderr) |

**Example:**

```bash
smelt serve --config server.toml
smelt serve --config server.toml --no-tui
```

See [Server Mode](#server-mode) below for configuration details.

---

### `smelt status`

Show status of a running or completed job.

```
Usage: smelt status [OPTIONS] [JOB_NAME]

Arguments:
  [JOB_NAME]  Job name to read (reads per-job state from `.smelt/runs/<job-name>/state.toml`).
              Omit to read legacy flat state for backward compat

Options:
      --dir <DIR>  Path to the project root directory (defaults to current directory) [default: .]
  -h, --help       Print help
  -V, --version    Print version
```

| Flag | Description |
|------|-------------|
| `--dir <DIR>` | Path to the project root directory (defaults to `.`) |

**Example:**

```bash
smelt status add-user-auth
smelt status add-user-auth --dir /path/to/project
```

---

### `smelt watch`

Watch a PR until it is merged or closed. Polls the forge API at a configurable interval and exits with code 0 on merge, 1 on close.

```
Usage: smelt watch [OPTIONS] <JOB_NAME>

Arguments:
  <JOB_NAME>  Job name to watch (must match job.name in the manifest used for smelt run)

Options:
      --interval-secs <INTERVAL_SECS>  Polling interval in seconds [default: 30]
  -h, --help                           Print help
  -V, --version                        Print version
```

| Flag | Description |
|------|-------------|
| `--interval-secs <INTERVAL_SECS>` | Polling interval in seconds (default: 30) |

**Example:**

```bash
smelt watch add-user-auth
smelt watch add-user-auth --interval-secs 10
```

---

## Server Mode

`smelt serve` runs a long-lived daemon that accepts and dispatches jobs. Configure it with a TOML file:

```toml
# Queue directory — smelt watches for .toml manifests dropped here
queue_dir = "/tmp/smelt-queue"

# Maximum parallel jobs
max_concurrent = 2

# Retry policy
retry_attempts = 3
retry_backoff_secs = 5

# HTTP API
[server]
host = "127.0.0.1"
port = 8765

# Optional: SSH worker pool for remote dispatch
# [[workers]]
# host = "worker1.example.com"
# user = "smelt"
# key_env = "WORKER_SSH_KEY"
# port = 22
```

### Job Submission

Jobs can be submitted two ways:

1. **Directory watch** — drop a `.toml` manifest file into `queue_dir/`
2. **HTTP API** — `POST /api/v1/jobs` with the manifest as the request body

### HTTP API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/v1/jobs` | Submit a new job (manifest TOML in request body) |
| `GET` | `/api/v1/jobs` | List all jobs with current status |
| `DELETE` | `/api/v1/jobs/:id` | Cancel a queued or running job |

### Queue Persistence

Queue state is automatically persisted to `queue_dir/.smelt-queue-state.toml` after every enqueue, complete, and cancel. On restart, `smelt serve` reloads this file and re-dispatches any jobs that were queued, retrying, or running at shutdown time — no operator intervention required.

### SSH Worker Pools

When `[[workers]]` entries are present in the server config, `smelt serve` dispatches jobs to remote hosts via SSH instead of running them locally. Jobs are round-robined across available workers; unreachable workers are skipped and the job is re-queued for the next available host.

### Live TUI

By default, `smelt serve` displays a live Ratatui terminal dashboard showing all jobs, their status, assigned worker, and elapsed time. Use `--no-tui` to disable the TUI and keep tracing output on stderr instead.

---

## Examples

The [`examples/`](examples/) directory contains reference manifests:

| File | Description |
|------|-------------|
| [`job-manifest.toml`](examples/job-manifest.toml) | Standard Docker runtime with multiple sessions |
| [`job-manifest-compose.toml`](examples/job-manifest-compose.toml) | Docker Compose runtime for multi-service environments |
| [`job-manifest-forge.toml`](examples/job-manifest-forge.toml) | Docker runtime with GitHub forge integration for automatic PR creation |
| [`job-manifest-k8s.toml`](examples/job-manifest-k8s.toml) | Kubernetes runtime targeting a remote cluster |
| [`agent-manifest.toml`](examples/agent-manifest.toml) | Minimal agent-focused manifest |
| [`bad-manifest.toml`](examples/bad-manifest.toml) | Intentionally invalid manifest for testing error handling |
| [`server.toml`](examples/server.toml) | Server daemon configuration for `smelt serve` |

Use `--dry-run` to validate any example without running it:

```bash
smelt run examples/job-manifest.toml --dry-run
```

---

## Ecosystem

Smelt is one layer in a three-part agentic development toolkit:

| Layer | Project | Role |
|-------|---------|------|
| **Infrastructure** | **Smelt** | Container provisioning (Docker, Compose, Kubernetes), environment isolation, forge delivery (PR creation), parallel job dispatch, SSH worker pools |
| **Orchestration** | **Assay** | Spec-driven sessions, dual-track quality gates, multi-agent coordination inside the container |
| **Context** | **Cupel** | Token-budgeted context window optimization (library consumed by Assay) |

**How they connect:** `smelt run` provisions a container and invokes `assay run` inside it. Assay manages the AI coding sessions using specs from the manifest. Cupel is a library that Assay uses internally to optimize context windows.

---

## Runtimes

Smelt supports three container runtimes, selected via the `environment.runtime` field in the job manifest:

| Runtime | Value | Use Case |
|---------|-------|----------|
| Docker | `"docker"` | Single-container jobs (default) |
| Docker Compose | `"compose"` | Multi-service environments (e.g., app + database) |
| Kubernetes | `"kubernetes"` | Remote cluster execution |

---

## Project State

Smelt stores per-job run state in `.smelt/runs/<job-name>/state.toml`. This file tracks the current phase, container ID, sessions, elapsed time, and PR URL (if forge is configured). Use `smelt status <job-name>` to inspect it.

---

## License

TBD
