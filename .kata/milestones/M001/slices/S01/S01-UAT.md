# S01: Prerequisites — Persistence & Rename — UAT

**Milestone:** M001
**Written:** 2026-03-16

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: This slice is a mechanical rename + persistence layer with no user-facing UI, no runtime services, and no external integrations. All behavior is verifiable through compilation, unit tests, schema snapshots, and grep-based rename verification.

## Preconditions

- Rust toolchain installed (cargo, clippy, rustfmt)
- `cargo insta` available for snapshot testing
- Repository checked out on the `kata/M001/S01` branch

## Smoke Test

`just ready` passes — confirms fmt, lint, all tests, and dependency audit are clean.

## Test Cases

### 1. Rename completeness

1. Run `rg "AgentSession" --type rust crates/`
2. **Expected:** Zero matches. The old type name does not appear anywhere in Rust source.

### 2. Persistence round-trip

1. Run `cargo test -p assay-core -- gate::session::tests::save_and_load_round_trip`
2. **Expected:** Test passes — a GateEvalContext saved to disk loads back with identical content.

### 3. List contexts sorted

1. Run `cargo test -p assay-core -- gate::session::tests::list_returns_sorted`
2. **Expected:** Test passes — multiple saved contexts are listed in sorted order by ID.

### 4. Schema snapshot updated

1. Run `cargo insta test -p assay-types`
2. **Expected:** No pending snapshots. The `gate-eval-context` snapshot is accepted and current.

### 5. MCP server compiles with write-through

1. Run `cargo build -p assay-mcp`
2. **Expected:** Clean compilation, no warnings. Write-through calls to save_context/load_context are wired in.

### 6. Full test suite

1. Run `cargo test -p assay-mcp`
2. **Expected:** All tests pass (91 unit + 27 integration).

## Edge Cases

### Path traversal rejection

1. Run `cargo test -p assay-core -- gate::session::tests::save_rejects_path_traversal`
2. **Expected:** Test passes — session IDs containing `../` are rejected.

### Not-found error

1. Run `cargo test -p assay-core -- gate::session::tests::load_not_found`
2. **Expected:** Test passes — loading a non-existent session returns `GateEvalContextNotFound`.

### List on empty directory

1. Run `cargo test -p assay-core -- gate::session::tests::list_empty`
2. **Expected:** Test passes — listing when no sessions exist returns an empty vec.

## Failure Signals

- `rg "AgentSession" --type rust crates/` returning any matches → rename incomplete
- `just ready` failing → compilation, lint, or test regression
- `cargo insta test -p assay-types` showing pending snapshots → schema snapshot not updated
- Persistence tests failing → save/load/list broken

## Requirements Proved By This UAT

- R001 — Persistence round-trip test + MCP compilation proves GateEvalContext persists to disk via write-through, with disk fallback for restart survival
- R002 — Zero grep matches + schema snapshot update proves complete rename from AgentSession to GateEvalContext

## Not Proven By This UAT

- R001 runtime restart survival is not proven by a live restart test — only by the disk fallback code path compiling and the persistence functions being tested in isolation. Full MCP protocol-level restart testing deferred to S07's E2E pipeline.
- Smelt concepts (RunManifest, RunExecutor) mentioned in R002 are not renamed in this slice — they don't exist yet and will be created with the correct names in S06.

## Notes for Tester

All test cases are automated and can be verified by running `just ready`. No manual steps required. The pre-existing `set_current_dir` race in two MCP tests may cause sporadic failures on full suite runs — rerun if those specific tests flake.
