# S05: Harness CLI & Scope Enforcement — UAT

**Milestone:** M002
**Written:** 2026-03-17

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: All outputs are deterministic generated config files and structured violation types — no live runtime, network, or agent interaction required. CLI dispatch, scope enforcement, and prompt generation are pure functions testable via unit/integration tests and snapshot comparison.

## Preconditions

- `just ready` passes (all tests green, fmt/lint/deny clean)
- Workspace builds with `cargo build -p assay-cli`

## Smoke Test

Run `cargo run -p assay-cli -- harness generate claude-code` — should produce .mcp.json and .claude/settings.json content to stdout without errors.

## Test Cases

### 1. Generate config for each adapter

1. `cargo run -p assay-cli -- harness generate claude-code`
2. `cargo run -p assay-cli -- harness generate codex`
3. `cargo run -p assay-cli -- harness generate opencode`
4. **Expected:** Each produces valid adapter-specific config to stdout. No errors. Output includes file names and byte counts on stderr.

### 2. Install writes config to disk

1. Create a temp directory
2. `cargo run -p assay-cli -- harness install claude-code --output-dir /tmp/test-harness`
3. **Expected:** Config files written to the directory. stderr reports file paths and counts.

### 3. Diff detects changes

1. Run install to create baseline config
2. Run `cargo run -p assay-cli -- harness diff claude-code`
3. **Expected:** Exit code 0 (no changes). No output.
4. Delete one of the generated files
5. Run diff again
6. **Expected:** Exit code 1. stderr reports the missing file as "added" (would be created).

### 4. Scope enforcement detects violations

1. `cargo test -p assay-harness -- scope` — all 9 tests pass
2. **Expected:** Tests cover: empty scope (no violations), in-scope files pass, out-of-scope files flagged, shared file conflicts detected, both-match returns SharedFileConflict, prompt generation with and without neighbors.

### 5. Backward compatibility

1. Parse a TOML manifest without `file_scope` or `shared_files` fields
2. **Expected:** Parses successfully via serde defaults. No errors.

## Edge Cases

### Unknown adapter name

1. `cargo run -p assay-cli -- harness generate unknown-adapter`
2. **Expected:** Error message listing valid adapters: "claude-code, codex, opencode"

### Empty file_scope means no restrictions

1. Call `check_scope([], [], ["any/file.rs"])`
2. **Expected:** Returns empty Vec (no violations)

## Failure Signals

- `just ready` fails (test regression, lint errors, or snapshot mismatches)
- `assay harness generate` panics or produces empty output
- Schema snapshots for ManifestSession/RunManifest don't include file_scope/shared_files
- Scope tests fail to detect out-of-scope violations or misclassify shared files

## Requirements Proved By This UAT

- R022 (Harness orchestration layer) — scope enforcement via globset patterns, multi-agent prompt generation, CLI dispatch to all three adapters with generate/install/update/diff lifecycle
- R024 (Additional harness adapters) — CLI dispatch to codex and opencode adapters (adapter logic proved by S04, CLI wiring proved here)

## Not Proven By This UAT

- Runtime scope enforcement during live agent execution (advisory only, no blocking)
- Integration with orchestrated parallel sessions (S06 scope)
- Real agent invocation with scope-aware config (manual UAT with real Claude Code/Codex/OpenCode)
- MCP tool exposure for orchestration (S06 scope)

## Notes for Tester

- `harness update` is intentionally identical to `harness install` — this is by design (D038), not a bug
- Scope enforcement is advisory: check_scope() returns violations but nothing prevents an agent from touching out-of-scope files at runtime
- All adapter snapshot tests are in assay-harness (not assay-cli) — CLI tests verify dispatch routing, not output format
