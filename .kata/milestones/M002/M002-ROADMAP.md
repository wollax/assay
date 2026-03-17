# M002: Real Assay Integration

**Vision:** `smelt run manifest.toml` provisions a container, runs a real `assay` binary that parses and executes the spec-based manifest Smelt generates, streams gate output to the terminal, and exits cleanly with a result branch on the host repo.

## Success Criteria

- `AssayInvoker` generates a `RunManifest` (`[[sessions]]` plural, `spec` = name reference) and per-session `Spec` TOML files that a real `assay` binary accepts without schema or parse errors
- `smelt run manifest.toml` runs the full pipeline — container provision → `.assay/` setup → spec file writes → assay execution → result collection → teardown — against a real `assay` binary
- Gate output from inside the container is visible on the terminal as `assay run` produces it (streaming, not buffered until exit)
- `assay run` exit code 2 (gate failures) is surfaced as a distinct outcome, not treated as a crash
- Pre-existing `run_without_dry_run_attempts_docker` test failure is resolved

## Key Risks / Unknowns

- **AssayInvoker contract** — `deny_unknown_fields` on both `RunManifest` and `Spec`; any extra or missing field causes a silent TOML parse failure; the current implementation has three violations (`[[session]]` key, inline `spec` description, and unknown fields `harness`/`timeout`). Must be right before any Docker test can prove integration.
- **Streaming output architecture** — `DockerProvider::exec()` buffers all output until exec completes; real `assay run` runs multi-minute AI sessions; adding streaming requires a new API surface on `RuntimeProvider` that all downstream consumers must handle.

## Proof Strategy

- **AssayInvoker contract** → retire in S01 by unit-testing that generated TOML round-trips through Smelt's own mirror types with zero unknown fields and correct key names (`sessions`, not `session`)
- **Streaming output architecture** → retire in S03 by adding `exec_streaming()` to `RuntimeProvider`/`DockerProvider` using bollard's streaming chunks, wiring Phase 7 of `execute_run()` to emit each chunk to stderr as it arrives

## Verification Classes

- Contract verification: unit tests in `assay.rs` asserting TOML key names, spec reference semantics, `deny_unknown_fields` safety, session name sanitization, and `--base-branch` flag presence
- Integration verification: `docker_lifecycle.rs` test with real `assay` binary built from source, injected via D040 pattern; assay must start its pipeline without emitting TOML/schema parse errors
- Operational verification: streaming exec delivers output incrementally (observable via test that sends chunks and checks arrival order); exit code 2 triggers correct phase transition in `execute_run()`
- UAT / human verification: end-to-end `smelt run` with a real manifest, real Docker, real `assay` binary, and real Claude API key — demonstrates complete pipeline producing a result branch

## Milestone Definition of Done

This milestone is complete only when all are true:

- `AssayInvoker` unit tests pass with correct `[[sessions]]` key, spec file format, and no unknown fields
- Integration test with real `assay` binary shows assay progressing past manifest/spec parse phase (no TOML schema errors in stderr)
- `execute_run()` Phase 5.5 (assay setup: config + specs dir + spec files) and Phase 6 (write run manifest) use the corrected `AssayInvoker` API
- `exec_streaming()` exists on `RuntimeProvider` and `DockerProvider`; Phase 7 uses it so gate output lines are printed as they arrive
- Exit code 2 from `assay run` is distinguished from exit code 1 — surfaces "gate failures" vs "pipeline error"
- Pre-existing `run_without_dry_run_attempts_docker` test failure is resolved
- D029 is superseded in `DECISIONS.md` with the validated contract (D043)

## Requirement Coverage

No `.kata/REQUIREMENTS.md` exists — operating in legacy compatibility mode. M002 covers entirely new capabilities not previously formalized.

- Covers: real Assay binary integration, contract-correct manifest + spec generation, streaming output, exit code semantics, `.assay/` setup lifecycle
- Partially covers: full end-to-end operational proof (depends on Claude API availability; automated tests prove up to assay manifest parsing; full run requires manual UAT)
- Leaves for later: Docker Compose runtime, PR/forge integration, multi-machine coordination
- Orphan risks: none (all M002 context items are accounted for)

## Slices

- [x] **S01: Fix AssayInvoker — Real Assay Contract** `risk:high` `depends:[]`
  > After this: `cargo test -p smelt-core` shows unit tests proving `AssayInvoker` generates `[[sessions]]` key, spec-name references (not inline descriptions), valid flat `Spec` TOML files with `[[criteria]]`, session name sanitization, and `--base-branch` flag — all verifiable without Docker.

- [x] **S02: Real Assay Binary + Production Wiring** `risk:high` `depends:[S01]`
  > After this: `cargo test -p smelt-cli --test docker_lifecycle test_real_assay_manifest_parsing` provisions a container, injects a real `assay` binary built from source, runs through the full smelt pipeline (provision → `.assay/` setup → spec writes → manifest write → assay exec), and asserts assay's stderr contains no TOML/schema parse errors — proving Smelt-generated files are accepted by the real binary.

- [ ] **S03: Streaming Assay Output** `risk:medium` `depends:[S02]`
  > After this: `smelt run` prints assay gate output lines to stderr as they are produced (not buffered until assay exits) — observable by running any `assay` command that emits output over time.

- [ ] **S04: Exit Code 2 + Result Collection Compatibility** `risk:low` `depends:[S03]`
  > After this: `smelt run` exits 2 (not 1) when `assay run` exits 2 (gate failures), reports the distinction clearly, and `ResultCollector` is verified to handle Assay's merge-to-base-branch behavior correctly — confirmed by unit test and the pre-existing `run_without_dry_run_attempts_docker` test resolved.

## Boundary Map

### S01 → S02

Produces:
- `SmeltRunManifest { sessions: Vec<SmeltManifestSession> }` — serializes to `[[sessions]]` TOML key
- `SmeltManifestSession { spec: String, name: Option<String>, depends_on: Vec<String> }` — `spec` is a spec-name reference, no `harness`/`timeout` fields
- `SmeltSpec { name: String, description: String, criteria: Vec<SmeltCriterion> }` — flat Assay spec file format
- `SmeltCriterion { name: String, description: String, cmd: Option<String> }` — wraps `SessionDef.harness` as the gate criterion command
- `AssayInvoker::build_run_manifest_toml(manifest: &JobManifest) -> String` — correct key, correct spec references
- `AssayInvoker::build_spec_toml(session: &SessionDef) -> String` — produces `Spec`-compatible TOML for one session
- `AssayInvoker::sanitize_session_name(name: &str) -> String` — replaces `/`, spaces, invalid filename chars with `-`
- `AssayInvoker::write_spec_file_to_container(provider, container, name, toml) -> Result<ExecHandle>` — base64-encodes + execs to `/workspace/.assay/specs/<sanitized-name>.toml`
- `AssayInvoker::build_ensure_specs_dir_command() -> Vec<String>` — `mkdir -p /workspace/.assay/specs`
- `AssayInvoker::build_write_assay_config_command(project_name: &str) -> Vec<String>` — `sh -c "if [ ! -f /workspace/.assay/config.toml ]; then mkdir -p /workspace/.assay && echo '...' | base64 -d > /workspace/.assay/config.toml; fi"` — idempotent, never clobbers existing `.assay/`
- `AssayInvoker::build_run_command(manifest)` extended: adds `--base-branch <manifest.job.base_ref>`
- D043 decision appended to `DECISIONS.md` superseding D029 with validated contract

Consumes:
- nothing (first slice)

### S02 → S03

Produces:
- `execute_run()` with corrected Phase 5.5 sequence: `ensure_assay_config` exec → `ensure_specs_dir` exec → per-session `write_spec_file_to_container` → `build_run_manifest_toml` + `write_manifest_to_container`
- Integration test `test_real_assay_manifest_parsing` using D039 phase-chaining + D040 binary injection pattern (real `assay` binary from `cargo build --bin assay`)
- `run_without_dry_run_attempts_docker` test fixed — assertion updated to accept exit code 127 ("assay not found in container") as a valid Docker-connected outcome

Consumes:
- All S01 `AssayInvoker` methods

### S03 → S04

Produces:
- `RuntimeProvider::exec_streaming(container, command, stdout_cb: impl FnMut(&str)) -> Result<ExecHandle>` — async method; calls `stdout_cb` with each output chunk as bollard delivers it
- `DockerProvider::exec_streaming()` implementation using bollard `AttachContainerOptions` + multiplexed log stream
- Phase 7 of `execute_run()` uses `exec_streaming` for `assay run`; all other phases (setup, write, collect) keep buffered `exec()`
- All existing `exec`-based tests unaffected (buffered variant unchanged)

Consumes:
- Working S02 `execute_run()` with Phase 5.5 wired

### S04 boundary

Produces:
- `execute_run()` maps `assay run` exit code 2 → `JobPhase::GatesFailed` (or equivalent signal), exits process with code 2 instead of treating as an error
- Stderr message: `"Assay complete — gate failures (exit 2)"` distinguishing from `"Assay complete — exit code: 1"`
- `ResultCollector::collect()` behavior verified against Assay's post-merge state: Assay merges session branches to base branch in the container; bind-mount means commits are visible on host; `ResultCollector` reads host `HEAD` unchanged — unit test confirms this invariant holds

Consumes:
- S03 streaming `execute_run()`
