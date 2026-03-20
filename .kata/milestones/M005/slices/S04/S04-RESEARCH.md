# S04: Gate-Gated PR Workflow — Research

**Date:** 2026-03-20

## Summary

S04 wires `assay pr create <milestone>` into the type/I/O foundation (S01) and cycle state machine (S02). It is entirely new code — no S03 output is needed (S04 depends on S01 and S02 only). The core pattern is: evaluate all chunks' required gates → if all pass, shell out to `gh pr create` → save PR number/URL to milestone TOML → transition milestone to `Complete`.

Three deliverables: (1) `assay-core/src/pr.rs` with `pr_check_milestone_gates` and `pr_create_if_gates_pass`, (2) `assay pr create` CLI subcommand, (3) `pr_create` MCP tool. The `Milestone` type needs two backward-compatible fields added (`pr_number`, `pr_url`). All of this follows patterns already established in S01/S02.

The primary risk is `gh` CLI availability and authenticated state — the function must degrade gracefully and return actionable errors. All gate evaluation reuses the existing `evaluate_all_gates` function from `assay-core::gate`. No new error variants are needed.

## Recommendation

Build `pr.rs` as a new module in `assay-core` with two pure-sync functions. Use the existing `milestone_scan`/`milestone_load`/`milestone_save` and `evaluate_all_gates` functions; do not duplicate logic. Shell out to `gh` with `std::process::Command` (D065, D008). Return `AssayError::Io` for all domain failures — no new error variant. The CLI command group (`assay pr create`) follows the exact same pattern as `assay milestone advance`.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|-----------------|------------|
| Gate evaluation for a chunk | `gate::evaluate_all_gates(&gates, working_dir, None, None)` | Already in `assay-core::gate`; accepts `GatesSpec`, returns `GateRunSummary` with `enforcement.required_failed` |
| Load a spec for a milestone chunk | `spec::load_spec_entry_with_diagnostics(slug, specs_dir)` | Returns `SpecEntry::Directory { gates, .. }` or error with suggestions |
| Atomic milestone save | `milestone::milestone_save(assay_dir, &milestone)` | tempfile + sync_all + rename; crash-safe |
| Shell out to git/gh | `std::process::Command::new("gh")` | D008/D065: CLI-first subprocess; same pattern as `worktree.rs` `git_command()` |
| Milestone phase transition | `milestone_phase_transition(&mut milestone, MilestoneStatus::Complete)` | Validates `Verify → Complete`; call after successful PR creation |

## Existing Code and Patterns

- `crates/assay-core/src/milestone/cycle.rs` — `cycle_advance` is the reference for the gate-eval → state-mutate → save pattern (steps 1-10). `pr_create_if_gates_pass` follows the same structure but checks ALL chunks instead of one active chunk.
- `crates/assay-core/src/worktree.rs` — `git_command(args, cwd)` pattern for shelling out to CLI tools. `gh` invocation follows the same `Command::new("gh").args([...]).current_dir(project_root).output()` shape. Parse `output.stdout` for the PR URL on success; map stderr to `AssayError::Io` on failure.
- `crates/assay-core/src/milestone/mod.rs` — `milestone_load`, `milestone_save`, `milestone_scan` I/O API. `pr_create_if_gates_pass` calls `milestone_load` → mutate `pr_number`/`pr_url`/`status` → `milestone_save`.
- `crates/assay-mcp/src/server.rs` — `cycle_advance` tool (lines 3375–3415) is the exact MCP tool pattern to follow for `pr_create`: `spawn_blocking` wrapper, `load_config` + `resolve_cwd`, map `Ok`/`Err` to `CallToolResult`.
- `crates/assay-cli/src/commands/milestone.rs` — `milestone_advance_cmd`: `anyhow::Result<i32>`, `eprintln!("Error: {e}")` + `return Ok(1)` for domain errors, `println!` for success. `pr_create_cmd` follows the same shape.
- `crates/assay-core/src/spec/mod.rs` — `load_spec_entry_with_diagnostics` is used in `cycle_advance` to load the spec for each chunk; reuse the same call in the gate-check loop.

## Milestone Type Changes

Add two fields to `Milestone` in `crates/assay-types/src/milestone.rs`:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub pr_number: Option<u64>,

#[serde(default, skip_serializing_if = "Option::is_none")]
pub pr_url: Option<String>,
```

These must be inserted before `created_at`/`updated_at` to keep TOML field ordering stable. Since `Milestone` uses `#[serde(deny_unknown_fields)]`, existing TOML files without these fields will deserialize correctly (fields absent = `None` via `serde(default)`). The schema snapshot for `milestone-schema` must be regenerated with `INSTA_UPDATE=always`.

**Struct literal cascade:** All `Milestone { ... }` struct literals in the workspace must be updated to include `pr_number: None, pr_url: None`. Known sites:
- `crates/assay-types/src/milestone.rs` — two test literals (`milestone_toml_roundtrip`, `milestone_minimal_toml_roundtrip`)
- `crates/assay-core/tests/milestone_io.rs` — `make_milestone` helper
- `crates/assay-core/tests/cycle.rs` — `make_milestone_with_status` helper and two inline `Milestone { ... }` literals

## `pr_check_milestone_gates` Design

```rust
pub struct ChunkGateFailure {
    pub chunk_slug: String,
    pub required_failed: usize,
}

pub fn pr_check_milestone_gates(
    assay_dir: &Path,
    specs_dir: &Path,
    working_dir: &Path,
    milestone_slug: &str,
) -> Result<Vec<ChunkGateFailure>> // Ok(vec![]) = all pass; Ok(non-empty) = some fail
```

The function:
1. Loads the milestone by slug
2. For each `ChunkRef` in `milestone.chunks` (in order): loads the spec, evaluates all gates, collects `required_failed > 0` entries
3. Returns `Ok(failures)` — empty = all pass, non-empty = some fail

Returns `AssayError::Io` only for I/O/parse errors (spec not found, TOML corrupt, etc.) — NOT for gate failures (those are returned as `ChunkGateFailure` items in the `Ok` Vec). This lets the caller format the failure list cleanly.

## `pr_create_if_gates_pass` Design

```rust
pub struct PrCreateResult {
    pub pr_number: u64,
    pub pr_url: String,
}

pub fn pr_create_if_gates_pass(
    assay_dir: &Path,
    specs_dir: &Path,
    working_dir: &Path,
    milestone_slug: &str,
    title: &str,
    body: Option<&str>,
) -> Result<PrCreateResult>
```

Algorithm:
1. `pr_check_milestone_gates` → if failures, return `AssayError::Io` with formatted list of failing chunks
2. Load milestone (needed for `pr_base`, `pr_branch`)
3. Resolve base branch: `milestone.pr_base` → fallback `"main"`
4. Build `gh pr create` args: `["pr", "create", "--title", title, "--base", base_branch]` + optional `["--body", body]`
5. Run `Command::new("gh").args([...]).current_dir(working_dir).output()`
6. On spawn failure (gh not in PATH): return `AssayError::Io` with "gh CLI not found — install from https://cli.github.com"
7. On non-zero exit: return `AssayError::Io` with stderr content
8. Parse stdout for the PR URL (gh outputs URL on its own line)
9. Parse PR number from URL (last path component, parse as u64)
10. Mutate milestone: `pr_number`, `pr_url`, `updated_at = Utc::now()`
11. If milestone is `Verify`, call `milestone_phase_transition(&mut milestone, MilestoneStatus::Complete)`
12. `milestone_save(assay_dir, &milestone)` atomically
13. Return `PrCreateResult { pr_number, pr_url }`

`PrCreateResult` and `ChunkGateFailure` are local types in `assay-core::pr`, not in `assay-types` (consistent with D051/D073 — response-only types stay local).

## `gh pr create` Output

`gh pr create` outputs the PR URL on the last line of stdout when successful:
```
https://github.com/owner/repo/pull/42
```
Parse: `stdout.trim().lines().last()` → the URL. PR number: `url.split('/').last()?.parse::<u64>()`.

When `gh` is not installed: `Command::spawn` returns `std::io::ErrorKind::NotFound`. When unauthenticated or missing remote: non-zero exit with stderr.

## CLI: `assay pr create`

New top-level `Pr` command group in `main.rs`:

```rust
Pr {
    #[command(subcommand)]
    command: commands::pr::PrCommand,
}
```

`PrCommand::Create { milestone: String, title: Option<String>, body: Option<String> }` in `commands/pr.rs`. The title defaults to `"feat: <milestone-name>"` if not provided. Calls `pr_create_if_gates_pass`. Prints "PR created: #N — <url>" on success. `eprintln!` + `Ok(1)` on failure.

## MCP: `pr_create` Tool

```rust
#[derive(Deserialize, JsonSchema)]
pub struct PrCreateParams {
    pub milestone_slug: String,
    pub title: String,
    #[serde(default)]
    pub body: Option<String>,
}
```

Follows the `cycle_advance` pattern exactly: `spawn_blocking` wrapping the sync `pr_create_if_gates_pass`. Returns JSON `{ pr_number, pr_url }` on success; `domain_error` on failure. Add one presence test `pr_create_tool_in_router`. Add `pr_create` to the doc comment at the top of `server.rs`.

## R046 Branch Naming Convention

R046 says chunk worktree branches follow `assay/<milestone-slug>/<chunk-slug>`. This is a convention for future worktree creation, not enforced by S04. The PR command uses `milestone.pr_branch` (if set) or relies on `gh` picking the current branch. **S04 does NOT need to change worktree branch naming** — R046 is a convention document, not a code change in this slice. The PR command opens a PR from whatever branch the user is currently on (or from `milestone.pr_branch`).

## Constraints

- No new `AssayError` variants — use `AssayError::Io` with descriptive messages (consistent with S01/S02 pattern, no new variant for this slice)
- `gh` output format (`https://github.com/owner/repo/pull/N`) is stable but should be parsed defensively (URL parse failure → fallback to raw stdout as `pr_url`, `pr_number = 0`)
- `spawn_blocking` required in MCP tool (sync `pr_create_if_gates_pass` must not block the async runtime)
- Schema snapshot for `Milestone` must be updated — all existing CI passes at workspace level (`cargo test --workspace`); standalone `-p assay-types` still requires `--features assay-types/orchestrate` workaround from S01
- Tool count assertions in MCP tests are presence-only (no count assertion) — adding `pr_create` won't break existing tests
- `Milestone` struct literals must all be updated after adding `pr_number`/`pr_url` — run `cargo test --workspace` after the type change to catch all sites

## Common Pitfalls

- **`gh` not on PATH causes spawn error, not non-zero exit** — handle `std::io::ErrorKind::NotFound` on spawn separately from non-zero exit codes; the error message must be actionable ("Install gh from https://cli.github.com")
- **`gh` authentication check** — `gh auth status` returns non-zero when not authenticated; but `gh pr create` will also fail with an actionable error from gh itself — don't try to pre-check auth, just surface gh's stderr
- **PR already exists** — if `milestone.pr_number.is_some()`, the PR was already created; either error with "PR already created: #N" or return the existing URL without creating again. Recommend: check first and return error with existing PR info.
- **`milestone_phase_transition` can only go Verify → Complete** — if milestone is still `InProgress` or `Draft` when PR is created, do NOT call the transition (only transition if status is `Verify`). The gates may pass even if cycle_advance was never called.
- **Struct literal cascade** — adding two fields to `Milestone` causes compile errors across ALL test files; fix all sites before running tests.
- **`INSTA_UPDATE=always`** for snapshot acceptance — use this non-interactively instead of `cargo insta review`.

## Open Risks

- `gh pr create --base` flag requires a valid base branch that exists as a remote ref; if the user's repo doesn't have a `main` branch (e.g., `master`), the default fallback will fail. Mitigate: surface `gh`'s error message directly (don't invent our own branch-detection logic for this case).
- Test coverage for actual `gh` invocation: unit tests cannot call real `gh`. Integration tests must use a fake `gh` script or skip the PR creation and test only the gate-check phase. Use the same approach as S02 cycle integration tests: test `pr_check_milestone_gates` independently; test `pr_create_if_gates_pass` with a mock `gh` binary (write a shell script to `PATH` that echoes a fake URL). Or: test only the gate-check path and mark the actual PR creation as UAT-only (document this explicitly).
- `gh` outputting the URL on stdout is conventional but not documented as a stable API. If `gh pr create --json` is available, consider `gh pr create --json number,url` which produces `{"number": N, "url": "..."}` — more stable to parse. This is worth investigating before implementation.

## gh JSON Output Option

`gh pr create` supports `--json` for machine-readable output:
```
gh pr create --title "feat: my-feature" --base main --json number,url
```
Output: `{"number":42,"url":"https://github.com/owner/repo/pull/42"}`

This is significantly safer than parsing the plain-text URL from stdout. **Recommended implementation**: use `--json number,url` and `serde_json::from_slice` to extract `number` and `url`. Avoids URL-parsing fragility entirely.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust/Cargo | — | no skill needed (standard toolchain) |
| gh CLI | — | none found; gh docs are adequate |

## Sources

- Milestone type + serde defaults pattern: `crates/assay-types/src/milestone.rs` (existing `pr_branch`/`pr_base` fields as exact reference)
- Gate evaluation entry point: `crates/assay-core/src/gate/mod.rs` `evaluate_all_gates`
- CLI shell-out pattern: `crates/assay-core/src/worktree.rs` `git_command` function
- MCP spawn_blocking pattern: `crates/assay-mcp/src/server.rs` `cycle_advance` (lines 3375–3415)
- Milestone I/O: `crates/assay-core/src/milestone/mod.rs`
- Cycle state machine: `crates/assay-core/src/milestone/cycle.rs`
- Error conventions: `crates/assay-core/src/error.rs` (AssayError::Io, no new variant)
- gh JSON output: `gh pr create --json number,url` (stable structured output; preferred over parsing plain URL)
- D065: PR creation shells out to gh CLI (Decisions Register)
- D008: git CLI-first subprocess pattern (Decisions Register)
