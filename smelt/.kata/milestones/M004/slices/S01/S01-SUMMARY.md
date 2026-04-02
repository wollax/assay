---
id: S01
parent: M004
milestone: M004
provides:
  - "`ComposeService` struct with `name`, `image`, and serde-flatten `extra: IndexMap<String, toml::Value>` (passthrough for all Compose keys)"
  - "`JobManifest.services: Vec<ComposeService>` with `#[serde(default)]` — backward-compatible, empty by default"
  - "`validate()` runtime allowlist: rejects anything other than `\"docker\"` or `\"compose\"`"
  - "`validate()` services-require-compose guard: rejects non-empty `services` when `runtime != \"compose\"`"
  - "`validate()` per-service name/image checks: rejects empty `name` or `image` when `runtime == \"compose\"`"
  - "10 new tests covering all validation invariants and the serde flatten boundary contract (131 total in smelt-core)"
  - "`indexmap` v2 as an explicit workspace dependency (serde feature enabled)"
requires: []
affects:
  - S02
  - S03
  - S04
key_files:
  - Cargo.toml
  - crates/smelt-core/Cargo.toml
  - crates/smelt-core/src/manifest.rs
  - crates/smelt-cli/tests/docker_lifecycle.rs
key_decisions:
  - "D073: ComposeService deliberately omits `#[serde(deny_unknown_fields)]` — passthrough is the design; extra keys captured via `#[serde(flatten)] extra: IndexMap<String, toml::Value>`"
  - "Runtime allowlist enforced at validate() time, not parse time — consistent with existing D018 collect-all-errors pattern"
  - "Services field validated conditionally: per-service name/image checks run only when `runtime == \"compose\"`; runtime and services-require-compose checks run unconditionally"
patterns_established:
  - "Compose service passthrough: arbitrary TOML keys → IndexMap<String, toml::Value> → YAML serialization path (used by S02 generate_compose_file)"
  - "`VALID_COMPOSE_MANIFEST` test constant covering all four TOML extra-field types (integer, boolean, array, string) — reusable across S02/S03 test suites"
observability_surfaces:
  - "`grep -n 'ComposeService\\|services:' crates/smelt-core/src/manifest.rs` — struct and field placement"
  - "`cargo test -p smelt-core --lib --test test_validate_runtime_unknown_rejected` — spot-check runtime validation path"
  - "All validation errors surface as descriptive strings in `SmeltError::Manifest { message }` per D018 — full collected error list visible to caller"
drill_down_paths:
  - .kata/milestones/M004/slices/S01/tasks/T01-SUMMARY.md
  - .kata/milestones/M004/slices/S01/tasks/T02-SUMMARY.md
duration: 15min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
---

# S01: Manifest Extension

**`ComposeService` passthrough struct and `Vec<ComposeService>` services field added to `JobManifest` with full validation — 131 tests pass, zero regressions across workspace.**

## What Happened

T01 established the data model: added `indexmap` v2 as a workspace dependency (serde feature), imported `IndexMap` in `manifest.rs`, defined `ComposeService` with `name`, `image`, and `#[serde(flatten)] extra: IndexMap<String, toml::Value>`, and added `#[serde(default)] pub services: Vec<ComposeService>` to `JobManifest`. The `deny_unknown_fields` attribute was intentionally omitted from `ComposeService` (D073) so that arbitrary Docker Compose keys flow through without a schema rejection. All 121 existing tests passed.

T02 extended `validate()` with three compose-specific rules: a runtime allowlist (only `"docker"` or `"compose"` accepted), a services-require-compose guard (non-empty `services` with a non-compose runtime is rejected), and per-service name/image non-empty checks (applied only when `runtime == "compose"`). The 10-test suite covers all boundary conditions including type-fidelity (integer, boolean, array, and string extra fields all round-trip correctly through TOML), the serde flatten exclusion contract (`extra` never contains `name` or `image`), and the compose-valid happy path. A compile fix in `docker_lifecycle.rs` (adding `services: vec![]` to a struct literal) was required to keep the workspace building after the new field was introduced.

## Verification

```
cargo test -p smelt-core --lib 2>&1 | grep -E "(test result|FAILED)"
  → test result: ok. 131 passed; 0 failed

cargo test --workspace 2>&1 | grep -E "(test result|FAILED)"
  → 8 crates, all "test result: ok. N passed; 0 failed" (no FAILED lines)
```

## Requirements Advanced

- R020 (Docker Compose runtime for multi-service environments) — S01 delivers the `ComposeService` type and `JobManifest.services` field that S02 (compose file generation), S03 (ComposeProvider lifecycle), and S04 (CLI dispatch) all depend on. The boundary contract specified in the M004 roadmap (name, image, passthrough via IndexMap, validation rules) is fully implemented and test-proven.

## Requirements Validated

- none — R020 requires live Docker integration (S03) before it can be moved to validated.

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

T02 added one test beyond the task plan's list (`test_validate_runtime_compose_valid` — the compose-valid happy path). This test was in the slice plan's Verification section but not in the task plan's step list; it was added proactively since it covers an important happy path.

A compile fix to `docker_lifecycle.rs` was not in the original task plan but was mechanically required: the `test_manifest_with_repo()` helper constructed `JobManifest` via struct literal and needed `services: vec![]` after the new field was added in T01.

## Known Limitations

- `ComposeService` serializes `extra` as `IndexMap<String, toml::Value>` — the YAML representation of `toml::Value` (via `serde_yaml`) is not yet tested. S02 will prove type fidelity in the YAML output via snapshot tests.
- `validate()` allows `runtime = "compose"` with zero services (per the spec) — an all-compose-no-services manifest is legal but somewhat unusual. This is intentional and documented.

## Follow-ups

- S02: implement `generate_compose_file()` consuming `JobManifest.services` and `environment.image`; snapshot-test TOML→YAML type fidelity
- S03: implement `ComposeProvider: RuntimeProvider` with internal `HashMap<ContainerId, ComposeProjectState>` lifecycle tracking
- S04: wire `runtime == "compose"` dispatch in `run.rs` and extend `print_execution_plan()` with `── Compose Services ──` section

## Files Created/Modified

- `Cargo.toml` — added `indexmap = { version = "2", features = ["serde"] }` to `[workspace.dependencies]`
- `crates/smelt-core/Cargo.toml` — added `indexmap.workspace = true` to `[dependencies]`
- `crates/smelt-core/src/manifest.rs` — `ComposeService` struct, `services` field on `JobManifest`, three validation blocks, `VALID_COMPOSE_MANIFEST` constant, 10 new tests
- `crates/smelt-cli/tests/docker_lifecycle.rs` — added `services: vec![]` to `test_manifest_with_repo()` struct literal

## Forward Intelligence

### What the next slice should know
- `toml::Value` serializes correctly from TOML but `serde_yaml` handles `toml::Value` variants differently — S02 must snapshot-test the YAML output for array, integer, and boolean values to prove type fidelity end-to-end. The `VALID_COMPOSE_MANIFEST` constant (two services, all four extra types) is the right input for those snapshot tests.
- `ComposeService.extra` is an `IndexMap` (ordered), not a `HashMap` — insertion order from TOML is preserved, which matters for deterministic YAML output in snapshot tests.
- `JobManifest` has `#[serde(deny_unknown_fields)]` but `ComposeService` does not — this asymmetry is intentional (D073). Do not add `deny_unknown_fields` to `ComposeService`.
- The `services` field is at the bottom of `JobManifest` — any struct-literal construction elsewhere in tests must include `services: vec![]` or the build will fail.

### What's fragile
- `toml::Value` → `serde_yaml` YAML serialization — integer, boolean, and array values are not yet round-trip proven through the full TOML→YAML chain. This is the primary S02 risk (already identified in the roadmap under "TOML → YAML type fidelity").

### Authoritative diagnostics
- `cargo test -p smelt-core --lib 2>&1 | grep -E "(test result|FAILED)"` — definitive pass/fail for manifest layer
- `grep -n 'ComposeService\|services:' crates/smelt-core/src/manifest.rs` — locate struct and field definitions quickly

### What assumptions changed
- No material assumptions changed during S01 execution. The serde flatten approach worked exactly as designed.
