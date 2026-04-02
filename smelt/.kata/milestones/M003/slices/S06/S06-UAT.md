# S06 UAT — End-to-End Smoke Test

**Milestone:** M003  
**Slice:** S06 — Integration Proof  
**Executed by:** Human (agent cannot exercise Docker + real GitHub)  
**Written:** 2026-03-21
**Status:** Ready for execution

---

## UAT Type

- UAT mode: live-runtime
- Why this mode is sufficient: The core value proposition — `smelt run` creates a real GitHub PR, `smelt watch` blocks until merge, exits 0 — requires real Docker, a real Assay binary, and real GitHub API calls. Automated tests cover individual components (mock HTTP, subprocess dry-run); only a human with real credentials can prove the full pipeline wires together end-to-end.

## Requirements Proved By This UAT

- R001 — Full primary user loop: `smelt init` → `smelt run` → result branch collected → GitHub PR created (URL printed) → `smelt status` shows PR section → `smelt watch` blocks → user merges → watch exits 0
- R005 — `smelt-core` library API stability: the fact that `smelt run` exercises `GitHubForge::create_pr()` and `DockerProvider::provision()` through the public library surface proves the API is usable end-to-end

## Not Proven By This UAT

- R005 external path-dependency embedding (proven separately by the `smelt-example` crate compilation in S05)
- Concurrent multi-job isolation under real load (proven by unit tests in S04, not repeated here)
- Rate-limit and ETag/conditional-request behaviour (proven by S03 unit tests against mock HTTP)
- `smelt list` output format correctness (proven by S04 unit tests)
- Cargo doc warning absence (proven by CI-level `RUSTDOCFLAGS="-D missing_docs"` checks in T01)

---

## Prerequisites

Before starting, confirm all of the following:

- [ ] Docker daemon is running (`docker info` exits 0)
- [ ] `GITHUB_TOKEN` is set and has **`pull_requests: write`** scope  
      (`echo $GITHUB_TOKEN` is non-empty)
- [ ] You control a GitHub repository with a `main` branch (the test will push branches and open a PR — use a sandbox repo)
- [ ] Smelt binary is built:  
      `cargo build --release --bin smelt` — or install via `cargo install --path crates/smelt-cli`
- [ ] `smelt --help` lists `init`, `run`, `status`, `watch`

---

## Step 1 — Init

Create a fresh working directory and generate a skeleton manifest:

```bash
mkdir /tmp/smelt-uat && cd /tmp/smelt-uat
smelt init
```

**Expected:**
- Exit code: **0**
- Output: `Created job-manifest.toml — edit and run with: smelt run job-manifest.toml`
- File `job-manifest.toml` exists in `/tmp/smelt-uat/`

Now edit `job-manifest.toml` to point at your test repo:

```toml
[job]
name     = "uat-test"          # pick a unique job name
repo     = "/path/to/your/local/clone"   # absolute path to a local git clone
base_ref = "main"

[environment]
runtime = "docker"
image   = "alpine:3"

[credentials]
provider = "anthropic"
model    = "claude-sonnet-4-20250514"

[merge]
strategy = "sequential"
order    = ["implement"]
target   = "main"

[forge]
provider  = "github"
repo      = "owner/repo"          # replace with your GitHub owner/repo
token_env = "GITHUB_TOKEN"
```

> **Note:** Keep the `[[session]]` block that `smelt init` generated and rename it `name = "implement"`.

---

## Step 2 — Dry Run

Validate the manifest without touching Docker or GitHub:

```bash
smelt run job-manifest.toml --dry-run
```

**Expected:**
- Exit code: **0**
- stdout contains `═══ Execution Plan ═══`
- stdout contains `uat-test` (the job name)
- stdout contains `── Forge ──`, `github`, `owner/repo`, `GITHUB_TOKEN`
- stdout contains `══ End Plan ══` (or `═══ End Plan ═══`)
- No Docker containers are started
- No GitHub API calls are made

---

## Step 3 — Live Run

Run the full pipeline (provisions Docker, runs agent sessions, collects results, creates PR):

```bash
smelt run job-manifest.toml
```

**Expected sequence (stderr):**
1. Container provisioning messages
2. Session `implement` starts — agent runs inside Docker
3. Session completes; result branch is collected
4. `PR created: https://github.com/owner/repo/pull/N` is printed
5. Process exits 0

**Pass criteria:**
- Exit code: **0**
- `PR created:` line appears in stderr with a valid `https://github.com/…` URL
- A new branch appears in your GitHub repository
- A pull request is open on GitHub targeting `main`

---

## Step 4 — Status

Inspect the recorded run state and PR metadata:

```bash
smelt status uat-test
```

**Expected:**
- Exit code: **0**
- Output contains `── Pull Request ──` section
- PR URL visible (matching the URL from Step 3)
- PR `state: open`
- CI status displayed (may be `Unknown` if the repo has no CI checks configured — see Troubleshooting)
- Review count displayed

---

## Step 5 — Watch

Block and poll until the PR is resolved:

```bash
smelt watch uat-test
```

**Expected:**
- Process blocks (does not exit immediately)
- Polling lines printed every ~30 seconds:  
  `[HH:MM:SS] PR #N — state: open | CI: pending | reviews: 0`
- Each poll line has a current timestamp
- Process continues running while PR is open

---

## Step 6 — Merge (human action)

While `smelt watch` is running in your terminal:

1. Open the PR URL from Step 3 in your browser
2. Merge the pull request on GitHub (use the "Merge pull request" button)

**Expected (back in the terminal running `smelt watch`):**
- Within ~30 seconds, a final line is printed:  
  `PR merged.`
- Process exits **0**

**Pass criteria:**
- `smelt watch` exits 0 after the merge
- No error messages in output

---

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| `PR already exists` error during Step 3 | A previous run created a PR for this job name that is still open | Change `job.name` to a new unique value (e.g. `uat-test-2`) or close/delete the existing PR |
| `CiStatus::Unknown` in `smelt status` / `smelt watch` output | Repo has no CI checks (branch protection rules, GitHub Actions workflows) configured | Expected and acceptable — does not block UAT; just means CI column shows Unknown |
| `smelt watch` reports "no state file" or cannot find the job | Job name used in watch command does not match `job.name` in manifest | Use exactly the value set in `[job] name = "…"` |
| `GITHUB_TOKEN not set` error | `GITHUB_TOKEN` env var is missing or empty | Run `export GITHUB_TOKEN=<your-pat>` before executing Steps 3–6 |
| Docker provisioning fails | Docker daemon is not running | Run `docker info` to verify; start Docker Desktop or the Docker daemon |
| `cargo build` fails on init skeleton | Missing credentials or network in container | This is a UAT of the pipeline wiring — agent output quality is separate from pipeline correctness |

---

## Expected UAT Result

All 6 steps complete without error:

| Step | Action | Pass Condition |
|------|--------|----------------|
| 1 | `smelt init` | Exit 0; `job-manifest.toml` created |
| 2 | `smelt run --dry-run` | Exit 0; Execution Plan + Forge section printed |
| 3 | `smelt run` (live) | Exit 0; `PR created: https://github.com/…` in stderr |
| 4 | `smelt status uat-test` | Exit 0; PR URL and `state: open` visible |
| 5 | `smelt watch uat-test` | Process blocks; polling lines printed every ~30s |
| 6 | Merge PR on GitHub | `smelt watch` prints `PR merged.` and exits 0 |

**R001 requirement satisfied** when all 6 steps pass: the full `smelt init` → `smelt run` → PR created → `smelt watch` → merge → exit 0 pipeline is confirmed end-to-end against a real GitHub repository.
