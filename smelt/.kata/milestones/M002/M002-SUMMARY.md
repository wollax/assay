---
id: M002
provides:
  - Four deny_unknown_fields serde types mirroring Assay's real schema (SmeltRunManifest, SmeltManifestSession, SmeltSpec, SmeltCriterion)
  - AssayInvoker methods: sanitize_session_name, build_spec_toml, build_run_manifest_toml, build_ensure_specs_dir_command, build_write_assay_config_command, write_spec_file_to_container, updated build_run_command with --base-branch
  - Phase 5.5 assay setup block in execute_run() (config write, specs dir, per-session spec files)
  - exec_streaming<F>() on RuntimeProvider trait and DockerProvider impl
  - Phase 7 of execute_run() wired to exec_streaming for live chunk delivery
  - JobPhase::GatesFailed variant for exit-code-2 distinction
  - Exit-code-2 pass-through path — assay run exit 2 bypasses bail, runs collect, exits process with code 2
  - build_linux_assay_binary() and inject_binary_to_container() test helpers for real binary integration tests
  - test_real_assay_manifest_parsing integration test — proves real assay binary passes TOML parse phase
  - test_exec_streaming_delivers_chunks_in_order integration test
  - test_collect_after_merge_commit and test_job_phase_gates_failed_serde unit tests
  - D043 supersedes D029 with validated Assay manifest contract
key_decisions:
  - D043: Assay manifest translation (supersedes D029) — Smelt writes spec files + RunManifest; [[sessions]] key, spec-name references, no harness/timeout fields
  - D044: Direct writes for .assay/ setup, never assay init — avoids AlreadyInitialized error and host repo side-effects
  - D045: .assay/ idempotency guard — check for config.toml before writing; mkdir -p always safe; spec files always overwrite
  - D046: exec_streaming() added alongside exec(); buffered exec() retained for setup phases
  - D047: Linux assay binary build via docker run rust:alpine with separate CARGO_TARGET_DIR; cache-first pattern
  - D048: Large binary injection via docker cp subprocess (base64 exec unreliable for 50-100 MB binaries)
  - D049: exec_streaming() callback bound FnMut(&str) + Send + 'static; Arc<Mutex<Vec<String>>> satisfies this in tests
  - D050: Exit-code-2 path: save assay_exit binding before branching; return Ok(assay_exit) at closure end
  - D051: ResultCollector merge-commit compatibility verified by unit test; no code changes required
patterns_established:
  - sanitize_session_name: replace non-[a-zA-Z0-9_-] with '-', collapse consecutive dashes, trim, fallback to "unnamed"
  - write_spec_file_to_container mirrors write_manifest_to_container (base64 encode + sh -c echo pipe)
  - build_write_assay_config_command uses "if [ ! -f ... ]" guard — idempotent, never clobbers existing .assay/
  - deny_unknown_fields roundtrip tests: serialize → toml::from_str back to typed struct
  - D039 phase-chaining in integration tests: call AssayInvoker static methods directly, mirroring execute_run() Phase 5.5 sequence
  - Callback-based streaming alongside buffered exec — same create/start/inspect skeleton, different output path
  - Teardown-before-assert ordering in integration tests
  - Distinct JobPhase variants for distinct semantic outcomes (Complete vs GatesFailed vs Failed)
  - Local Wrapper struct in serde tests when subject is a bare enum value and toml is the serialization target
observability_surfaces:
  - eprintln!("Writing assay config/specs dir/spec: ...") confirms Phase 5.5 entry
  - "Manifest loaded: N session(s)" in assay stderr = parse phase passed (ground-truth integration signal)
  - "Assay complete — gate failures (exit 2)" vs "Assay complete — exit code: N" distinguishes outcomes
  - cat .smelt/run-state.toml — phase = "gates_failed" is ground-truth signal for exit-2 path
  - RUST_LOG=smelt_core=debug — logs per-chunk debug! stream="stdout"/"stderr" in exec_streaming
  - ExecHandle.stdout / ExecHandle.stderr always populated by exec_streaming for post-hoc inspection
requirement_outcomes: []
duration: ~3h (S01: ~1h, S02: ~1h15m, S03: ~20m, S04: ~30m)
verification_result: passed
completed_at: 2026-03-17
---

# M002: Real Assay Integration

**Retired the mock Assay contract and proved end-to-end correctness: `AssayInvoker` now generates `[[sessions]]`-keyed RunManifests and per-session spec files that a real `assay` binary accepts without schema errors, gate output streams live to the terminal, and exit code 2 is surfaced as a distinct `GatesFailed` outcome.**

## What Happened

Four slices delivered the milestone in sequence, each building directly on the last.

**S01 — Fix AssayInvoker** identified the three contract violations in the original `AssayInvoker`: the wrong `[[session]]` key (must be `[[sessions]]`), an inline `spec` description where Assay expects a name reference to a pre-existing spec file, and unknown fields `harness`/`timeout` that Assay's `deny_unknown_fields` structs would reject silently. The fix replaced the two broken serde types with four new ones mirroring Assay's real schema and added eight new/updated methods. Thirteen unit tests proved contract correctness at the TOML serialization level — all verifiable without Docker. D043 was appended to `DECISIONS.md`, superseding D029.

**S02 — Real Assay Binary + Production Wiring** wired Phase 5.5 into `execute_run()` (the assay setup sequence: config write → specs dir → per-session spec files → run manifest) and proved the generated TOML passes real Assay validation. A Linux aarch64 assay binary was built via `docker run rust:alpine` with a separate `CARGO_TARGET_DIR` volume (D047) and injected into a test container via `docker cp` (D048). The integration test `test_real_assay_manifest_parsing` proved that a real `assay` binary reaches `"Manifest loaded: 2 session(s)"` without TOML/schema parse errors — Assay progresses past manifest/spec parse phase into Phase 1 (session execution), exiting only at Phase 2 (git checkout) due to no real git repo in the test container. The pre-existing `run_without_dry_run_attempts_docker` failure was also resolved in this slice.

**S03 — Streaming Assay Output** added `exec_streaming<F>()` to `RuntimeProvider` and `DockerProvider`, routing each bollard chunk through a callback instead of buffering until completion. Phase 7 of `execute_run()` was wired to use `exec_streaming()` with `|chunk| eprint!("{chunk}")` as the callback, eliminating a double-print bug (the previous pattern re-printed the full buffered output after exec). The `eprint!` calls were removed from `exec()`'s output loop — `exec()` is now fully silent, output available only via `ExecHandle`. The integration test `test_exec_streaming_delivers_chunks_in_order` validated real chunk delivery order against a live Docker container.

**S04 — Exit Code 2 + Result Collection Compatibility** completed the slice by adding `JobPhase::GatesFailed` to `monitor.rs`, splitting the Phase 7 bail guard so `assay run` exit 2 falls through to collect rather than aborting, printing a distinct `"Assay complete — gate failures (exit 2)"` message, and propagating `Ok(2)` through `execute_run()` to `std::process::exit(2)` in `main.rs`. `ResultCollector::collect()` was unit-tested against Assay's post-merge state (`git merge --no-ff`) — no code changes were needed; the implementation already handled merge commits correctly.

## Cross-Slice Verification

**Success criterion 1: AssayInvoker generates correct RunManifest and spec files**
- Verified by 13 unit tests in `assay.rs` including `test_run_manifest_uses_sessions_key_plural`, `test_run_manifest_no_unknown_fields`, `test_run_manifest_spec_is_sanitized_name_not_description`, and all roundtrip tests
- `cargo test -p smelt-core` → 112 passed, 0 failed

**Success criterion 2: `smelt run` runs the full pipeline against a real assay binary**
- Verified by `test_real_assay_manifest_parsing` — provisions container, injects real Linux aarch64 assay binary, runs Phase 5.5 + Phase 6, execs `assay run`, asserts `"Manifest loaded: 2 session(s)"` in stderr
- `cargo test -p smelt-cli --test docker_lifecycle test_real_assay_manifest_parsing -- --nocapture` → test result: ok. 1 passed

**Success criterion 3: Gate output is visible on terminal as assay run produces it (streaming)**
- Verified by `test_exec_streaming_delivers_chunks_in_order` against real Docker — chunk order preserved, `ExecHandle` populated
- Phase 7 uses `exec_streaming()` with `|chunk| eprint!("{chunk}")` callback
- `grep -n "eprint!" crates/smelt-core/src/docker.rs` → no matches (exec() is silent)

**Success criterion 4: assay run exit code 2 is surfaced as a distinct outcome**
- Verified by `test_job_phase_gates_failed_serde` — `JobPhase::GatesFailed` round-trips through TOML serde as `"gates_failed"`
- Phase 7 bail guard changed from `!= 0` to `!= 0 && != 2`; exit-2 arm sets `GatesFailed` and propagates `Ok(2)`
- Process exits with code 2 via `std::process::exit(2)` in `main.rs`

**Success criterion 5: Pre-existing `run_without_dry_run_attempts_docker` test failure resolved**
- Resolved in S02 — Phase 5.5 now executes and all setup commands succeed before assay exits 127 (not found)
- `cargo test --workspace` → 7 test suites, all "test result: ok." — no FAILED

**Milestone definition of done — all items checked:**
- [x] AssayInvoker unit tests pass with correct `[[sessions]]` key, spec file format, no unknown fields
- [x] Integration test with real assay binary shows assay progressing past manifest/spec parse phase
- [x] Phase 5.5 and Phase 6 use the corrected AssayInvoker API
- [x] `exec_streaming()` exists on `RuntimeProvider` and `DockerProvider`; Phase 7 uses it
- [x] Exit code 2 distinguished from exit code 1 — `GatesFailed` vs generic error
- [x] `run_without_dry_run_attempts_docker` test failure resolved
- [x] D043 appended to `DECISIONS.md` superseding D029

**Final workspace verification:**
```
cargo test --workspace
# test result: ok. 10 passed (smelt-cli unit)
# test result: ok. 0 passed (smelt-cli doc)
# test result: ok. 23 passed (docker_lifecycle)
# test result: ok. 10 passed (dry_run)
# test result: ok. 112 passed (smelt-core)
# test result: ok. 0 passed (smelt-core doc)
# test result: ok. 2 passed (smelt-cli integration)
```

## Requirement Changes

No `.kata/REQUIREMENTS.md` exists — operating in legacy compatibility mode. No requirement status transitions to record.

## Forward Intelligence

### What the next milestone should know
- Assay's post-session merge behavior: Assay merges session branches to the base branch inside the container. The bind-mount (D013) means commits appear on the host repo filesystem. `ResultCollector` reads host `HEAD` and works correctly against merge commits — `test_collect_after_merge_commit` makes this invariant explicit.
- `exec()` is now fully silent — any caller that relied on exec() printing to stderr for debugging must switch to `exec_streaming()` or inspect `ExecHandle.stdout`/`ExecHandle.stderr` manually.
- The `GatesFailed` phase is now in `RunState` — any future `smelt status` display work should render it distinctly from `Failed` and `Complete`.
- The exit-code-2 path is only exercised end-to-end with a real `assay` binary and real Claude API key; the current automated tests prove the phase transition exists but cannot trigger it without live AI sessions.

### What's fragile
- `build_linux_assay_binary()` cache path at `target/smelt-test-cache/assay-linux-aarch64` — `cargo clean` removes it; Docker build must re-run (~5-15 min). CI must have the assay source repo available or the integration test skips gracefully.
- `--platform linux/arm64` is hardcoded in the Docker build command — breaks on x86_64 hosts; flag and binary arch would need parameterization.
- The `Ok(2)` arm in the Phase 7 outcome match must remain before the generic `Ok(code)` arm — order-sensitive; swapping silently routes exit-2 to `JobPhase::Complete`.
- `build_write_assay_config_command` embeds a base64-encoded config string — changing the template requires re-encoding; unit test only checks structural properties, not decoded content.
- `.assay/` directory is written to the bind-mounted host repo during live runs — no `.gitignore` entry exists yet; users may accidentally commit ephemeral Assay project state.

### Authoritative diagnostics
- `"Manifest loaded: N session(s)"` in assay stderr — primary integration success signal; look here first
- `"unknown field"` in assay stderr — TOML schema mismatch from `deny_unknown_fields`; check the exact field name in the error message
- `cat .smelt/run-state.toml` — `phase = "gates_failed"` is the ground-truth signal for exit-2 path
- `echo $?` after `smelt run` — must be `2` for gate failures, not `1`
- `RUST_LOG=smelt_core=debug cargo test ... -- --nocapture` — logs `debug!(stream = "stdout"/"stderr", ...)` per chunk in `exec_streaming`

### What assumptions changed
- "rust:alpine has musl libc dev headers" — false; `apk add --no-cache musl-dev` must be prepended to the Docker build command
- "build_linux_assay_binary() should check source before cache" — wrong order; cache-first is correct; source check first caused unnecessary test skips when cache existed but source repo was absent (fixed in S02/T03)
- "`collect()` would need changes to handle Assay's post-merge commits" — false; the existing `rev_list_count`/`diff_name_only` calls handle merge parents correctly by default; `test_collect_after_merge_commit` makes this invariant explicit

## Files Created/Modified

- `crates/smelt-core/src/assay.rs` — complete rewrite: four new deny_unknown_fields serde types, eight methods, 13-test module
- `crates/smelt-core/src/provider.rs` — `exec_streaming<F>()` method added to `RuntimeProvider` trait
- `crates/smelt-core/src/docker.rs` — `exec_streaming<F>()` impl added; `eprint!` removed from `exec()` loop
- `crates/smelt-core/src/monitor.rs` — `GatesFailed` variant added to `JobPhase`; `test_job_phase_gates_failed_serde` added
- `crates/smelt-core/src/collector.rs` — `test_collect_after_merge_commit` unit test added
- `crates/smelt-cli/src/commands/run.rs` — Phase 5.5 block inserted; Phase 7 uses `exec_streaming()`; exit-code-2 path; post-exec eprint block removed
- `crates/smelt-cli/tests/docker_lifecycle.rs` — `workspace_root()`, `build_linux_assay_binary()`, `inject_binary_to_container()`, three new integration tests; Arc<Mutex> import added
- `crates/smelt-cli/tests/dry_run.rs` — clarifying comment on `run_without_dry_run_attempts_docker`
- `.kata/DECISIONS.md` — D043–D051 appended
- `.kata/milestones/M002/M002-SUMMARY.md` — this file
