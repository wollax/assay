# S01: Scaffold, Manifest & Dry-Run CLI — UAT

**Milestone:** M001
**Written:** 2026-03-17

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: S01 is a dry-run-only slice with no runtime side effects — all behavior is manifest parsing, validation, and output formatting, fully verifiable via CLI invocations and test assertions

## Preconditions

- Rust toolchain installed (cargo/rustc)
- `cargo build --workspace` succeeds
- Example manifests exist at `examples/job-manifest.toml` and `examples/bad-manifest.toml`

## Smoke Test

Run `cargo run -- run examples/job-manifest.toml --dry-run` — should print a structured execution plan showing job name, image, 3 sessions, and merge config.

## Test Cases

### 1. Valid manifest produces complete execution plan

1. Run `cargo run -- run examples/job-manifest.toml --dry-run`
2. **Expected:** Output contains job name (`add-user-auth`), repository, base ref, image (`node:20-slim`), resources (cpu, memory), 3 sessions (frontend, backend, integration) with their specs/timeouts, merge config (strategy, target, order), and credential status

### 2. Invalid manifest produces clear validation errors

1. Run `cargo run -- run examples/bad-manifest.toml --dry-run`
2. **Expected:** Exit code 1, stderr contains validation errors for: empty job name, empty image, zero timeout, duplicate session name, unknown dependency, empty merge target, unknown merge order entry

### 3. Credential resolution shows status without exposing values

1. Set `ANTHROPIC_API_KEY=test-value` in environment
2. Run `cargo run -- run examples/job-manifest.toml --dry-run`
3. **Expected:** Output shows `env:ANTHROPIC_API_KEY → resolved` — the actual value `test-value` never appears anywhere in stdout or stderr

### 4. Missing credentials are reported

1. Unset `ANTHROPIC_API_KEY` from environment
2. Run `cargo run -- run examples/job-manifest.toml --dry-run`
3. **Expected:** Output shows `env:ANTHROPIC_API_KEY → MISSING`

### 5. Nonexistent manifest file

1. Run `cargo run -- run nonexistent.toml --dry-run`
2. **Expected:** Exit code 1, error message mentions the file path

### 6. Non-dry-run mode placeholder

1. Run `cargo run -- run examples/job-manifest.toml`
2. **Expected:** Exit code 1, message indicates Docker execution is not yet implemented

## Edge Cases

### Unknown TOML fields rejected

1. Create a manifest with an extra field like `[job]\nname = "test"\nrepo = "x"\nbase_ref = "main"\nfoo = "bar"`
2. Run `cargo run -- run that-manifest.toml --dry-run`
3. **Expected:** TOML parse error mentioning the unknown field `foo`

### Session dependency cycle detected

1. Create a manifest where session A depends on B and B depends on A
2. Run `cargo run -- run cycle-manifest.toml --dry-run`
3. **Expected:** Validation error mentioning circular dependency

## Failure Signals

- `cargo build --workspace` produces warnings or errors
- `cargo test --workspace` has any failures
- Dry-run output is missing sections (no sessions, no credentials, no merge config)
- Credential values appear anywhere in output
- Invalid manifest exits 0 instead of 1
- Validation reports only the first error instead of all errors

## Requirements Proved By This UAT

- No `.kata/REQUIREMENTS.md` — requirements tracked via milestone roadmap

## Not Proven By This UAT

- Docker container provisioning and teardown (S02)
- Real runtime execution of any commands inside containers
- Repo mounting and Assay invocation (S03)
- Result collection and branch output (S04)
- Live job monitoring or timeout enforcement (S05)

## Notes for Tester

- The `assert_cmd::Command::cargo_bin` deprecation warning in test output is a known upstream issue — not a test failure
- All 71 tests (58 smelt-core + 13 smelt-cli) should pass with `cargo test --workspace`
- Set `SMELT_LOG=info` for structured tracing output during dry-run invocations
