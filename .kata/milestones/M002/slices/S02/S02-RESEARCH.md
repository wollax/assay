# S02: Real Assay Binary + Production Wiring — Research

**Date:** 2026-03-17

## Summary

S02 has two distinct jobs: (1) wire Phase 5.5 into `execute_run()` so the assay config, specs directory, and per-session spec files are written before the run manifest; and (2) add an integration test that runs a real `assay` binary to prove the generated files are accepted without TOML/schema parse errors.

The Phase 5.5 wiring is straightforward — every method needed was delivered by S01 (`build_write_assay_config_command`, `build_ensure_specs_dir_command`, `write_spec_file_to_container`, `write_manifest_to_container`). The only work is calling them in the right sequence inside `execute_run()`, before the existing Phase 6 manifest write.

The integration test is more nuanced. The assay binary at `/Users/wollax/Git/personal/assay/target/debug/assay` is a **macOS arm64 Mach-O executable** — it cannot run inside a Linux Alpine container. The test must build a Linux aarch64 assay binary before it can inject it. The recommended approach is to build it via `docker run rust:alpine` at test setup time, caching the result to a known host path so the build only happens once. If Docker build fails or is unavailable, the test skips gracefully (same `docker_provider_or_skip()` pattern).

**`run_without_dry_run_attempts_docker` in `dry_run.rs` currently passes** — verified by running the test suite. After Phase 5.5 wiring, `smelt run examples/job-manifest.toml` will additionally exec the config write and spec write steps before reaching `assay run`; all these use `sh -c "..."` / `mkdir` which exist in alpine:3. The existing assertion (no "not implemented" in stderr) will remain valid. No change is strictly required, but the plan notes it should accept exit 127 as a valid outcome — that's a documentation/comment update to the test, not a behavioral fix.

## Recommendation

Implement S02 in three tasks:

**T01 — Phase 5.5 wiring in `execute_run()`**: Add the setup sequence to `run.rs` between Phase 5 (provision) and Phase 6 (write manifest). The sequence:
1. `exec(build_write_assay_config_command(&manifest.job.name))` — idempotent config write
2. `exec(build_ensure_specs_dir_command())` — `mkdir -p /workspace/.assay/specs`
3. For each session: `write_spec_file_to_container(&provider, &container, sanitized_name, spec_toml)`
4. Existing Phase 6: `build_run_manifest_toml()` + `write_manifest_to_container()`

**T02 — Linux assay binary build helper**: Add a test-only helper function (or a `tests/support/` module) that produces a Linux aarch64 `assay` binary via `docker run rust:alpine`. Cache the output binary to `target/smelt-test-cache/assay-linux-aarch64`. Return a `PathBuf` or `None` (skip signal) if Docker or source unavailable.

**T03 — Integration test `test_real_assay_manifest_parsing`**: Using D039 phase-chaining and D040 injection, provision a container, inject the Linux assay binary at `/usr/local/bin/assay`, run Phase 5.5 + Phase 6 directly, then exec `assay run`. Assert that assay's stderr contains `"Manifest loaded:"` and does NOT contain `"unknown field"`, `"TOML parse"`, or `"No Assay project found"`. Assay will exit non-zero after parsing (no Claude API key in test container) — that's expected; the test only proves the parse phase passes.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Writing config/spec files into container | All S01 methods: `build_write_assay_config_command`, `build_ensure_specs_dir_command`, `write_spec_file_to_container`, `write_manifest_to_container` | Fully implemented, tested, ready to wire |
| Phase-chaining in integration tests | D039 pattern: direct phase method calls, no `run_with_cancellation()` | Already used in `test_full_e2e_pipeline` |
| Binary injection at test setup | D040 pattern: base64-encode + `echo pipe \| base64 -d > /usr/local/bin/assay && chmod +x` | Already proven in `test_full_e2e_pipeline`; extend for Linux binary bytes |
| Skipping without Docker daemon | `docker_provider_or_skip()` | Used in all Docker tests; extend with binary-availability check |
| TOML spec serialization | `toml::to_string_pretty()` | Already a dependency, used in `build_spec_toml()` |

## Existing Code and Patterns

- `crates/smelt-cli/src/commands/run.rs` — `execute_run()` currently has Phase 5 (provision) → Phase 6 (write manifest) → Phase 7 (exec assay). Phase 5.5 inserts between Phase 5 and Phase 6. The error handling pattern for each phase is uniform: return early with teardown on error (see the Phase 6 `write_result` block). Follow the same pattern for each Phase 5.5 exec.
- `crates/smelt-cli/tests/docker_lifecycle.rs` — `test_full_e2e_pipeline` is the best model for `test_real_assay_manifest_parsing`: provision → inject binary → write manifest → exec `build_run_command()` → teardown. The real-assay test adds Phase 5.5 steps and changes the assertion from "exit 0 + commit created" to "assay stderr contains 'Manifest loaded'" (since assay will fail past parse phase).
- `crates/smelt-core/src/assay.rs` — All S01 methods are available. `write_spec_file_to_container` takes `&impl RuntimeProvider` (not dyn), matching how `execute_run()` uses `provider`. The sanitized name used in `SmeltManifestSession.spec` is `sanitize_session_name(&session.name)` — same function used in `write_spec_file_to_container` — so names are guaranteed consistent.
- `crates/smelt-core/src/docker.rs` — `DockerProvider::exec()` streams output chunks to stderr via `eprint!` as they arrive (using bollard's `Attached` stream). The buffered return value in `ExecHandle.stdout`/`ExecHandle.stderr` accumulates everything. Phase 5.5 uses buffered exec for all setup commands (correct — they're fast/short).

## Constraints

- **macOS binary cannot run in Linux container**: `/Users/wollax/Git/personal/assay/target/debug/assay` is Mach-O arm64. Alpine containers run aarch64 Linux. The test must build or obtain a Linux binary — there is no shortcut.
- **No Linux cross-compilation target installed**: `rustup target list --installed` shows only `aarch64-apple-darwin`. Installing `aarch64-unknown-linux-musl` and a musl linker is an option but requires host setup changes. Docker-based build is self-contained.
- **D002 (firm)**: No `assay-types` crate dependency in Smelt. The integration test cannot import Assay types to validate the TOML — it must rely on assay's own stderr to prove parse success. The assertion `"Manifest loaded: N session(s)"` in assay's stderr proves the manifest parsed and validated without TOML/schema errors.
- **Assay requires `.assay/` directory at project root**: `assay run` checks for `.assay/` on startup and bails with `"No Assay project found. Run \`assay init\` first."` if missing. Phase 5.5 must write `config.toml` (via `build_write_assay_config_command`) before `assay run` is called. The idempotency guard in the command (`if [ ! -f /workspace/.assay/config.toml ]`) means running on a repo that already has `.assay/config.toml` is safe.
- **`config.specs_dir` defaults to `"specs/"` in assay-types**: Smelt writes spec files to `/workspace/.assay/specs/<name>.toml`. The minimal config Smelt writes does NOT override `specs_dir`, so it defaults to `"specs/"`. Assay loads specs from `root/.assay/specs/<spec_name>.toml`. These paths are consistent — no mismatch.
- **Container needs `sh` and `base64`**: All exec commands use `sh -c "..."` with `base64 -d`. Alpine:3 ships both. No additional `apk add` needed for Phase 5.5.

## Common Pitfalls

- **Forgetting to add Phase 5.5 error paths**: Each Phase 5.5 exec can fail (non-zero exit code). `write_spec_file_to_container` returns `crate::Result<ExecHandle>`. Failure must trigger teardown just like Phase 6's `write_result` block. Missing this leaves containers orphaned.
- **Confusing Phase 5.5 ordering**: Config write must come before specs dir (`mkdir -p` under `.assay/` requires `.assay/` to be creatable, but `mkdir -p` handles that). Specs dir must exist before spec file writes. Run manifest write comes last. Order: config → specs_dir → spec files (per-session) → run manifest.
- **Test binary path is macOS Mach-O**: Using `std::fs::read("/Users/wollax/Git/personal/assay/target/debug/assay")` and injecting those bytes will produce a container binary that immediately segfaults or fails to exec with "exec format error". Must be the Linux aarch64 build.
- **Assay exits non-zero past parse phase in test**: The integration test container won't have `ANTHROPIC_API_KEY` or Claude network access. Assay will parse the manifest and spec files successfully, then fail at the SpecLoad or AgentLaunch stage. The test should only assert on the parse-phase stderr messages and NOT assert `exit_code == 0`.
- **`depends_on` reference semantics in assay's validate()**: Assay validates `depends_on` references against "effective names" (`name` if set, else `spec`). Our `SmeltManifestSession.depends_on` is populated from `SessionDef.depends_on` which references session names as-is. If the manifest writer references session names that don't match the effective name in the generated manifest, validation fails. Since `SmeltManifestSession.name = Some(session.name.clone())` and `spec = sanitize_session_name(&session.name)`, the effective name is `session.name` (not the sanitized version). So `depends_on` values must be the original session names from `SessionDef.depends_on`, not sanitized — and that's what `build_run_manifest_toml()` does (copies `s.depends_on` unchanged). This is correct, but the test should use a session whose `depends_on` references its display name, not the sanitized spec name.
- **Docker build cache for Linux assay binary**: The `docker run rust:alpine cargo build` approach will re-download crates on every test run if no volume cache is set up. Use `-v $HOME/.cargo/registry:/usr/local/cargo/registry` to cache crate downloads, or cache the output binary at a stable path and skip rebuild if it exists.

## Open Risks

- **Linux binary build time**: A cold Docker build of the assay binary (downloading Rust toolchain + all crates) can take 5–15 minutes. With registry volume caching, subsequent builds are ~30 seconds. Without CI caching, this makes the test unpleasant to run. Mitigate by caching the built binary at `target/smelt-test-cache/assay-linux-aarch64` (gitignored) and skipping rebuild if it exists and is newer than assay source files.
- **rust:alpine image pull time**: The first test run on a clean machine needs to pull `rust:alpine`. This is ~600 MB. On CI with a warm Docker layer cache this is fast; locally it can take minutes. The test should still skip (not fail) if Docker is unavailable.
- **Assay binary size for injection via base64 exec**: Debug Linux binaries can be 50–100 MB. The D040 pattern uses `echo '<base64>' | base64 -d > /usr/local/bin/assay` — a 100 MB binary produces a 133 MB base64 string. Docker's exec stdin transfer at that size may be very slow or fail. **Alternative**: use Docker's `copy_to_container` (bollard `upload_to_container`) with a tar stream, which is the correct way to inject large files. Or mount the host assay binary path into the container via an additional bind mount — cleanest option.
- **`project_root()` inside assay**: `assay run` resolves the project root from the current working directory. Smelt's exec sets `working_dir: "/workspace"` (D027). This means assay will look for `.assay/` at `/workspace/.assay/` — which is exactly where Phase 5.5 writes it. Consistent.
- **`run_without_dry_run_attempts_docker` behavior after Phase 5.5**: After wiring, the test run of `smelt run examples/job-manifest.toml` will execute the Phase 5.5 steps (config write, specs dir, spec files) before reaching `assay run`. All these succeed in alpine:3. Then `assay run` exits 127 (not found). The existing test assertion (no "not implemented" in stderr) still passes. No behavioral regression expected.

## Assay Startup Sequence (Critical for Test Assertion Design)

When `assay run /tmp/smelt-manifest.toml` is executed:
1. `project_root()` — detects `/workspace` as project root (via CWD)
2. Checks `/workspace/.assay/` exists — bails with `"No Assay project found"` if missing
3. Loads `/workspace/.assay/config.toml` — bails on parse error
4. Computes `specs_dir = /workspace/.assay/specs/` (from `config.specs_dir` default `"specs/"`)
5. Prints `"Loading manifest: /tmp/smelt-manifest.toml"` to stderr
6. Parses manifest TOML — bails with `"ManifestParse"` error on unknown fields / wrong keys
7. Validates manifest — bails with `"ManifestValidation"` on empty spec, unknown dep refs
8. Prints `"Manifest loaded: N session(s)"` — **this is the integration test's success signal**
9. Loads spec files from `specs_dir/<session.spec>.toml` — fails with `SpecNotFound` if missing
10. Runs pipeline (worktree, harness, agent) — fails without Claude API key

**Test assertion**: stderr contains `"Manifest loaded: N session(s)"` → parse phase passed. stderr does NOT contain `"No Assay project found"`, `"unknown field"`, `"ManifestParse"`, `"ManifestValidation"` → no contract violations.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust / Cargo | — | none found (standard) |
| Docker / bollard | — | none found (project-specific) |
| Assay | — | internal project; no public skill |

## Sources

- `execute()` function in assay-cli: checks `.assay/` dir, loads config, loads manifest, prints "Manifest loaded" (source: `/Users/wollax/Git/personal/assay/crates/assay-cli/src/commands/run.rs`)
- `assay-types::Config` struct: `project_name` required, `specs_dir` defaults to `"specs/"`, both `deny_unknown_fields` (source: `/Users/wollax/Git/personal/assay/crates/assay-types/src/lib.rs`)
- `assay-types::RunManifest` and `ManifestSession`: `deny_unknown_fields` confirmed, `sessions` (plural) key, `spec` is name reference (source: `/Users/wollax/Git/personal/assay/crates/assay-types/src/manifest.rs`)
- `assay-core::manifest::validate()`: checks depends_on against effective names (`name` if set, else `spec`); Smelt's `SmeltManifestSession.name = Some(session.name)` so effective name = `session.name` (source: `/Users/wollax/Git/personal/assay/crates/assay-core/src/manifest.rs`)
- Spec loading path: `load_spec_entry(slug, specs_dir)` tries `<specs_dir>/<slug>.toml` (flat) and `<specs_dir>/<slug>/gates.toml` (directory) (source: `/Users/wollax/Git/personal/assay/crates/assay-core/src/spec/mod.rs`)
- Binary format: `/Users/wollax/Git/personal/assay/target/debug/assay` is Mach-O 64-bit arm64 (macOS) — confirmed via `file` command; cannot run in Linux Alpine containers
- Alpine containers on this host run `aarch64` Linux — confirmed via `docker run alpine:3 uname -m`
- No Linux Rust targets installed — confirmed via `rustup target list --installed` (only `aarch64-apple-darwin`)
- Current test suite status: all 152 tests pass (`cargo test --workspace`); `run_without_dry_run_attempts_docker` passes as-is
- Phase 5.5 wiring gap: `execute_run()` currently calls `build_run_manifest_toml()` + `write_manifest_to_container()` in Phase 6 without any Phase 5.5 spec/config setup; assay config and spec files are never written today
- DockerProvider large file injection: bollard supports `upload_to_container` with tar stream — preferred over base64 exec for files > a few MB (source: `crates/smelt-core/src/docker.rs` and bollard docs)
