# S04: CLI Integration + Dry-Run — UAT

**Milestone:** M004
**Written:** 2026-03-21

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: All S04 acceptance criteria are machine-verifiable via CLI output inspection and automated tests. The `--dry-run` path exercises the compose services display without Docker; the `cargo test --workspace` suite covers dispatch correctness and regression safety. No UI, human experience judgment, or live-runtime-only signal is required.

## Preconditions

- Rust toolchain installed; `cargo build -p smelt-cli` succeeds
- `examples/job-manifest-compose.toml` and `examples/job-manifest.toml` present at repo root
- No Docker daemon required for dry-run tests

## Smoke Test

```bash
cargo run --bin smelt -- run examples/job-manifest-compose.toml --dry-run
```

Expected: exits 0; stdout contains `── Compose Services ──` and `postgres  postgres:16-alpine`.

## Test Cases

### 1. Compose dry-run shows services section

```bash
cargo run --bin smelt -- run examples/job-manifest-compose.toml --dry-run
```

1. Observe stdout.
2. **Expected:** Exit 0. Output contains `── Compose Services ──`, `postgres`, and `postgres:16-alpine`. The `── Environment ──` section shows `Runtime: compose`.

### 2. Docker dry-run does NOT show services section

```bash
cargo run --bin smelt -- run examples/job-manifest.toml --dry-run
```

1. Observe stdout.
2. **Expected:** Exit 0. Output does NOT contain `── Compose Services ──`. The `── Environment ──` section shows `Runtime: docker`.

### 3. Workspace test suite — zero regressions

```bash
cargo test --workspace 2>&1 | grep -E "(test result|FAILED)"
```

1. **Expected:** All `test result:` lines show `0 failed`. No `FAILED` lines.

### 4. Dry-run integration tests specifically

```bash
cargo test -p smelt-cli --test dry_run 2>&1 | grep -E "(test result|FAILED)"
```

1. **Expected:** `test result: ok. 15 passed; 0 failed`.

## Edge Cases

### Unknown runtime in manifest

1. Create a manifest with `runtime = "k8s"` (or any unsupported value).
2. Run `smelt run <manifest>` (non-dry-run).
3. **Expected:** `validate()` rejects the manifest before reaching Phase 3; error message lists `runtime` as invalid. No Docker call is made.

### Docker manifest with no services — regression check

1. Run `cargo run --bin smelt -- run examples/job-manifest.toml --dry-run`.
2. **Expected:** Exit 0; output unchanged from pre-M004 behavior; no `── Compose Services ──` section.

## Failure Signals

- `── Compose Services ──` absent from compose manifest dry-run output → `print_execution_plan()` condition wrong; check `!manifest.services.is_empty()` guard in `run.rs`
- `── Compose Services ──` present in docker manifest dry-run output → guard not checking `runtime`; section should be data-driven by `manifest.services`
- `FAILED` in `cargo test --workspace` output → regression in existing tests; check `dry_run` and `compose_lifecycle` suites first
- `unsupported runtime` error for `runtime = "compose"` → AnyProvider match arm missing; grep `run.rs` for `AnyProvider` dispatch block

## Requirements Proved By This UAT

- R020 — `smelt run manifest.toml` with `runtime = "compose"` dispatches to `ComposeProvider`; `--dry-run` shows `── Compose Services ──`; `runtime = "docker"` path is unchanged; all acceptance criteria for CLI dispatch and dry-run UX are machine-verified

## Not Proven By This UAT

- Live end-to-end `smelt run examples/job-manifest-compose.toml` (non-dry-run) provisioning a real Postgres container, running Assay, and tearing down — requires Docker daemon + ANTHROPIC_API_KEY. This was proven by S03 integration tests (`test_compose_provision_exec_teardown`, `test_compose_healthcheck_wait_postgres`); the CLI dispatch path is identical to what S03 tests.
- Ctrl+C teardown of compose stack during live run — proven by S03 signal handling; S04 adds no new teardown logic.

## Notes for Tester

- All acceptance criteria for this slice are fully automated; human UAT is informational only.
- The `examples/job-manifest-compose.toml` uses `image = "alpine:3"` for the smelt-agent — sufficient for dry-run. For a live run, replace with the actual Assay agent image and set a valid `repo` path.
- `job.repo = "."` in the example manifest means "current directory" — fine for local testing but should be an absolute path in CI.
