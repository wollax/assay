# Audit: Dogfooding assay and using it to build itself - focus specifically on the core assay functionality, aimed at the solo developer running one session at a time. Spec-driven development, gated PRs, git management, etc.

**Date:** March 31, 2026
**Goal:** Dogfooding assay and using it to build itself - focus specifically on the core assay functionality, aimed at the solo developer running one session at a time. Spec-driven development, gated PRs, git management, etc.
**Codebase:** assay / `/Users/wollax/Git/personal/assay`

---

## Strengths

### The Core Loop Exists and Is Used

The project has one dogfooding spec at `.assay/specs/self-check.toml` that runs `cargo fmt --check`, `cargo clippy`, `cargo test`, and `cargo deny`. This spec is exactly the kind of "live gate" assay is designed to enforce. The `just ready` recipe mirrors it, and the pre-push git hook runs `just ready` — meaning every push is gated by the same quality criteria the tool itself defines. The loop is closed at the local-dev level.

### Git Hook Infrastructure Is Solid

`.githooks/pre-commit` runs fmt-check + clippy + plugin version consistency on every commit. `.githooks/pre-push` runs the full `just ready` suite. Setup is a single `just setup` invocation. This is exactly the pattern a solo developer would want — cheap fast checks on commit, full checks on push.

### `assay gate run` Is Fully Functional

`crates/assay-core/src/gate/mod.rs` implements `evaluate`, `evaluate_all`, and `evaluate_all_gates` with: process-group kill on timeout, independent head+tail stream truncation per stdout/stderr, exit-code 127/126 enrichment, advisory/required enforcement split, and history persistence. The implementation is production-quality. The CLI surface (`assay gate run`, `assay gate run --all`, `--json`, `--verbose`, `--timeout`) covers the solo-dev use case.

### Gate History Is Persisted and Queryable

`assay gate history <name>` and `--last` / detail view work end-to-end. `assay_core::history` saves `GateRunRecord` JSONL files per spec with pruning (`max_history`). This gives a solo developer an audit trail — the ability to see when gates started failing and correlate with commits.

### Gated PR Creation Works

`assay pr create <milestone>` in `crates/assay-core/src/pr.rs` checks all milestone chunk gates before calling `gh pr create`. Idempotency guard prevents duplicate PRs. PR body template rendering with `{milestone_name}`, `{chunk_list}`, `{gate_summary}` placeholders is implemented. The `pr_base`, `pr_labels`, and `pr_reviewers` fields on milestone TOML give fine-grained control.

### Stop Hook Enforces Gates in Claude Code

`plugins/claude-code/scripts/cycle-stop-check.sh` is a well-engineered Stop hook: 5-guard safety pattern (jq, infinite-loop, mode-off, no-.assay/, no-binary), cycle-aware chunk selection, warn/enforce/off modes, and structured JSON blocking responses. This is the critical "agent can't declare done until gates pass" mechanism. It works.

### Planning Wizard + MCP Tool Coverage

`assay plan` (TTY wizard) and the `milestone_create` / `spec_create` MCP tools provide two entry points for planning — interactive and programmatic. The Claude Code plugin's `/assay:plan` skill ties these together for an agent-driven planning flow. The `spec_list`, `spec_get`, `gate_run`, `cycle_status`, `cycle_advance`, `chunk_status` MCP tools give the agent visibility into the full cycle state without needing the CLI directly.

### Worktree Isolation Is Built In

`assay worktree create <spec>` creates a `git worktree` per spec with a namespaced branch. This gives a solo developer proper branch-per-chunk isolation without manual git ceremony. `worktree_cleanup` tears down cleanly. The MCP server exposes the same surface so an agent can manage its own worktree lifecycle.

### JSON Schema and Type System Are First-Class

`schemas/` contains generated JSON Schemas for all public types. `just schemas-check` is in CI (`.github/workflows/ci.yml.disabled`). `schemars` derives are on all serializable types in `assay-types`. This means the MCP server's tool inputs are always schema-validated, and external tooling can introspect the data model.

### Test Coverage Is Substantial

`assay-core/src/gate/mod.rs` has ~800 lines of inline `#[cfg(test)]` covering every evaluation path. Integration tests in `crates/assay-core/tests/` cover PR status, gate history, milestone I/O, pipeline spans, and orchestration. `crates/assay-mcp/tests/` covers MCP handlers and the signal server. Snapshot tests (insta) lock harness config generation. The test pyramid is healthy for a Rust codebase.

---

## Gaps

### CI Workflow Is Disabled

`.github/workflows/ci.yml.disabled` — the CI workflow is intentionally disabled (filename). Only `release.yml` runs on GitHub. This means pushes to main and PRs are not automatically validated. For a tool that enforces gated PRs, not running CI on its own PRs is a trust gap. The release workflow builds cross-platform binaries but does not run `just ready` first.

**Impact:** Solo developer loses the safety net of "CI caught something I didn't run locally." PRs to this repo are not guarded by the same gates the tool enforces.

### Only One Self-Check Spec, No Directory-Format Specs

`.assay/specs/self-check.toml` is a legacy flat spec. The codebase's own preferred format is directory-based (`specs/<name>/gates.toml` + `spec.toml`). There are no directory-format specs for assay itself. The `FeatureSpec` type (`feature_spec.rs`) with IEEE 830-style requirement fields (`Obligation`, `SpecStatus`, `Requirement`) exists but is not used in any real spec for the project. The `self-check.toml` does not exercise `path`-based criteria, `AgentReport` criteria, or the `depends` field — key features that are untested via dogfooding.

**Impact:** The most expressive spec format is not used to describe the tool's own features. When bugs appear in directory-spec loading or `FeatureSpec` parsing, there is no dogfooding regression surface to catch them.

### No Milestone Exists for the Project Itself

`.assay/milestones/` does not exist — there are no milestone TOML files. The milestone, chunk, and cycle machinery (`assay plan`, `assay milestone list`, `assay pr create`) is entirely non-exercised on the assay repo. The `assay pr create` flow (which is the primary "gated PR" feature) has never been run against this project.

**Impact:** The most end-to-end feature — plan → implement → gate-check → PR create — is not dogfooded. Bugs in milestone TOML I/O, cycle advance logic, or `gh` integration can only be caught by tests, not by day-to-day use.

### `assay plan` Wizard Is Not Used

There is no milestone, so the `assay plan` wizard has never been exercised on this project. The wizard is TTY-only and tests only confirm it exits with code 1 in non-TTY. The actual conversational flow — from milestone name through chunk criteria collection — is untested via dogfooding. If the wizard produces malformed TOML, creates the wrong directory structure, or confuses the user, there is no feedback loop.

### `assay worktree` Is Not Used for Assay's Own Development

Despite the worktree subsystem being fully implemented, assay is developed on a single branch (observed from checkpoint: `primary` agent working in `/Users/wollax/Git/personal/assay` directly). The chunk-per-worktree isolation pattern isn't applied to the project's own development. This means the worktree lifecycle (create → work → cleanup) isn't exercised against real git operations on this repo.

### The TUI (`assay-tui`) Is a Scaffold

`crates/assay-tui/src/main.rs` and `crates/assay-tui/src/app.rs` exist but the TUI is described in the README as scaffold-level. The TUI tests in `crates/assay-tui/tests/` do test UI components (spec_browser, trace_viewer, wizard_round_trip), but the TUI is not a working product a solo developer can actually use. For a solo-dev dogfooding workflow, this is a gap: there's no visual dashboard to observe cycle state without hitting the CLI.

### PR Status Panel in TUI Is Disconnected

`crates/assay-tui/tests/pr_status_panel.rs` tests a PR status panel, and `crates/assay-core/src/pr.rs` implements `pr_status_poll`. But there is no CLI command surfacing PR status — no `assay pr status` command. A solo developer who has run `assay pr create` has no CLI-native way to poll the open PR's state (CI checks, review decision, merge status) without running `gh` directly.

### Release Workflow Does Not Gate on `just ready`

`.github/workflows/release.yml` builds binaries for 4 targets on tag push but does not run `just ready` before building. A tag can be pushed with failing tests, and release binaries will be produced. For a tool that enforces quality gates, this is a consistency gap.

### `AssayError` Overuses `Io` Variant

`crates/assay-core/src/pr.rs` uses `AssayError::Io` for all error paths including gate failures, milestone not found, and JSON parse errors. Multiple comments note "D065: consistent with S01/S02 patterns." This creates diagnostic opacity: a caller receiving `AssayError::Io` for "gates failed" gets the same type as one receiving it for "file not found." For a tool that agents interact with via MCP, structured error types matter for self-correction.

### `AgentReport` Criteria Are Skipped, Not Evaluated, in CLI `gate run`

In `handle_gate_run`, `AgentReport` criteria are counted as `skipped`. The session-based evaluation path (`gate_report` + `gate_finalize` MCP tools, or `gate_evaluate` single-call tool) exists but is never exercised in assay's own `self-check.toml`. The `self-check.toml` has one `AgentReport` criterion (`code-quality-review`) with `enforcement = "advisory"` — but running `assay gate run self-check` will skip it. There is no path to exercise this criterion in a solo-dev workflow without MCP.

---

## Next Steps

Prioritized actions directly usable as input to `/kata plan`:

### P1 — Enable CI (1 task)
Re-enable `.github/workflows/ci.yml.disabled` → rename to `ci.yml`. Add `just ready` as a step in `release.yml` before the build matrix. This closes the "assay doesn't eat its own cooking on PRs" gap immediately.

### P2 — Create a Milestone for Assay's Next Feature (1–3 tasks)
Run `assay plan` (or `milestone_create` via MCP) to create a real milestone with 2–4 chunks for a real upcoming feature. The goal is to exercise the full `plan → gate → pr create` loop against this repo. Candidate feature: enabling CI + adding directory-format specs for this project. This is the highest-value dogfooding action.

### P3 — Migrate `self-check` to Directory-Format Spec (1 task)
Convert `.assay/specs/self-check.toml` → `.assay/specs/self-check/gates.toml` + `spec.toml`. Add a `path`-based criterion (e.g. `path = "schemas/spec.schema.json"` to verify schema generation is up to date). Add a `depends = []` entry to exercise DAG loading. This exercises the directory-spec format under real use.

### P4 — Add `assay pr status` CLI Command (1–2 tasks)
Add `assay pr status <milestone>` that calls `pr_status_poll(pr_number)` and prints state, CI check counts, and review decision. This closes the "created a PR, now what?" gap for the solo developer without requiring `gh pr view` knowledge.

### P5 — Use Worktrees for Chunk Development (workflow, no code)
Document in `AGENTS.md` and the Claude Code `CLAUDE.md` plugin that `assay worktree create <chunk>` should be run at the start of each chunk. This is a workflow convention change, not a code change, but it exercises the most complex git-management feature under real conditions and will surface any bugs in `worktree create/cleanup` sooner.

### P6 — Wire `AgentReport` Criterion to MCP `gate_evaluate` in Solo Flow (2–3 tasks)
Add documentation (and optionally a Claude Code skill `/assay:agent-review`) that shows a solo developer how to trigger `gate_evaluate` for a spec containing `AgentReport` criteria. The `self-check.toml` `code-quality-review` criterion is already there — it just needs a path to be invoked. This closes the "advisory AI review is invisible in normal gate run" gap.

### P7 — Add `assay pr merge` Command (2 tasks)
After `pr_status_poll`, implement `assay pr merge <milestone>` that checks state == Merged, runs `git pull main`, and transitions the milestone to `Complete`. This completes the PR lifecycle CLI surface and closes the gap between GitHub state and local milestone TOML state.

---

*Generated by /audit — read-only recce, no code was modified.*
