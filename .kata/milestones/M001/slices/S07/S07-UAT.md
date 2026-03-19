# S07: End-to-End Pipeline — UAT

**Milestone:** M001
**Written:** 2026-03-16

## UAT Type

- UAT mode: mixed (artifact-driven for automated tests + live-runtime for manual Claude Code verification)
- Why this mode is sufficient: Pipeline correctness (stage sequencing, error handling, structured output) is fully verified by automated tests. Real agent invocation requires a Claude Code installation and API key, which makes it a manual verification step.

## Preconditions

- Assay built successfully (`cargo build`)
- A valid spec exists in `.assay/specs/` (e.g., `example.toml`)
- For full live test: Claude Code CLI installed and `ANTHROPIC_API_KEY` set
- Git repository initialized with at least one commit

## Smoke Test

Run `assay run --help` — confirms the subcommand is registered and displays `--timeout`, `--json`, `--base-branch` flags.

## Test Cases

### 1. Pipeline unit tests pass

1. Run `cargo test -p assay-core -- pipeline`
2. **Expected:** 18 tests pass covering stage display, error construction, config defaults, outcome variants, harness profile building, agent launch failure, empty sessions, spec-not-found, and worktree-create failure.

### 2. MCP tool tests pass

1. Run `cargo test -p assay-mcp -- run_manifest`
2. **Expected:** 5 tests pass covering param deserialization (minimal and full), schema generation, tool router listing, and missing manifest error.

### 3. CLI tests pass

1. Run `cargo test -p assay-cli`
2. **Expected:** 4 tests pass covering clap arg parsing and JSON serialization of success/error responses.

### 4. Full suite green

1. Run `just ready`
2. **Expected:** All checks pass (fmt, clippy, all tests, deny).

### 5. CLI run with missing manifest (error path)

1. Run `assay run nonexistent.toml`
2. **Expected:** Error message indicating the manifest file was not found. Exit code 1.

### 6. CLI run with fixture manifest (live — requires Claude Code)

1. Create a minimal manifest TOML pointing to a real spec
2. Run `assay run manifest.toml --json`
3. **Expected:** Pipeline sequences through stages (progress printed to stderr), returns structured JSON to stdout with session outcomes and stage timings. If Claude Code is not installed, fails at AgentLaunch stage with recovery guidance.

## Edge Cases

### Empty sessions array

1. Create a manifest with `sessions = []`
2. Run `assay run empty.toml --json`
3. **Expected:** Validation error — empty sessions array rejected with actionable message.

### Spec not found

1. Create a manifest referencing a nonexistent spec slug
2. Run `assay run bad-spec.toml`
3. **Expected:** Pipeline fails at SpecLoad stage with error identifying the missing spec and recovery guidance.

### Agent timeout

1. Verified by unit test: `launch_agent()` with thread-based timeout fires after configured duration
2. **Expected:** PipelineError with AgentLaunch stage, timed_out flag, and recovery guidance about increasing timeout.

## Failure Signals

- Any `just ready` check failing
- `assay run --help` not showing the subcommand
- MCP `run_manifest` tool not appearing in tool router listing
- Pipeline errors missing stage context or recovery guidance
- CLI `--json` output not parseable as valid JSON

## Requirements Proved By This UAT

- R017 (single-agent E2E pipeline) — automated tests prove stage sequencing, error handling, and structured output. Full live runtime proof requires manual test case 6 with real Claude Code.
- R018 (pipeline as MCP tool) — `run_manifest` tool registers in router, schema generates correctly, spawn_blocking wrapping compiles and is tested.
- R019 (pipeline structured errors) — every tested failure path produces PipelineError with stage, message, recovery, and elapsed time.

## Not Proven By This UAT

- Real Claude Code `--print` invocation — requires installed Claude Code and API key. The pipeline is wired to invoke it, but actual subprocess behavior (output parsing, exit codes from a real agent run) is not covered by automated tests.
- Real gate evaluation after agent completion — requires a real agent to produce code changes that can be evaluated.
- Real merge check after successful gate evaluation — requires a completed agent run with passing gates.
- Multi-session manifest execution — `run_manifest` iterates sessions, but all tests use single-session or empty manifests.

## Notes for Tester

- The `launch_agent()` function invokes `claude --print` as a subprocess. If Claude Code is not installed, the pipeline will fail at AgentLaunch stage with a clear error — this is expected and validates the error handling path.
- Pre-existing flaky test `session_create_happy_path` in assay-mcp occasionally fails under parallel execution. Not related to S07.
- The `--json` flag on the CLI produces machine-readable output suitable for piping to `jq` for inspection.
