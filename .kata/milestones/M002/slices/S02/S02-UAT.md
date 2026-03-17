# S02: Real Assay Binary + Production Wiring — UAT

**Milestone:** M002
**Written:** 2026-03-17

## UAT Type

- UAT mode: live-runtime
- Why this mode is sufficient: The slice's primary claim is that Smelt-generated TOML files are accepted by a real `assay` binary — this is a binary compatibility fact that only a live runtime (Docker + real binary) can prove. Unit tests verify structure; only live execution against a real binary with `deny_unknown_fields` confirms no schema violations.

## Preconditions

- Docker daemon is running and accessible on the test host
- `target/smelt-test-cache/assay-linux-aarch64` exists (built via `test_build_linux_assay_binary_caches` or by setting `ASSAY_SOURCE_DIR` and running that test)
- `cargo build -p smelt-cli` succeeds (all Phase 5.5 code compiles)
- Working directory: `/Users/wollax/Git/personal/smelt`

## Smoke Test

```bash
cargo test -p smelt-cli --test docker_lifecycle test_real_assay_manifest_parsing -- --nocapture
```

Expected: test passes; assay stderr contains `"Manifest loaded: 2 session(s)"`.

## Test Cases

### 1. Phase 5.5 steps execute before assay in a real container

```bash
cargo test -p smelt-cli --test dry_run run_without_dry_run_attempts_docker -- --nocapture
```

1. Observe test output for "Writing assay config...", "Writing specs dir...", "Writing spec: ..." lines
2. Confirm test still passes (exit 0)
3. **Expected:** Phase 5.5 steps appear in output; assay exits 127 (not found); test reports `ok`

### 2. Real assay binary accepts Smelt-generated TOML without schema errors

```bash
cargo test -p smelt-cli --test docker_lifecycle test_real_assay_manifest_parsing -- --nocapture
```

1. Test provisions an Alpine container
2. Injects cached Linux aarch64 assay binary via `docker cp`
3. Runs Phase 5.5 (config write → specs dir → per-session spec writes) + Phase 6 (manifest write)
4. Executes `assay run` and captures stdout/stderr
5. **Expected:**
   - `assay stderr` contains `"Manifest loaded: 2 session(s)"`
   - `assay stderr` does NOT contain `"No Assay project found"`, `"unknown field"`, `"ManifestParse"`, or `"ManifestValidation"`
   - Test reports `ok`

### 3. Linux assay binary builder caches the binary

```bash
# Clean cache to force rebuild
rm -f target/smelt-test-cache/assay-linux-aarch64

# Build from source (requires ASSAY_SOURCE_DIR)
ASSAY_SOURCE_DIR=<path-to-assay-repo> \
  cargo test -p smelt-cli --test docker_lifecycle test_build_linux_assay_binary_caches -- --nocapture
```

1. **Expected:** Docker build runs (~5–15 min), binary appears at `target/smelt-test-cache/assay-linux-aarch64`
2. Subsequent call returns instantly (cache hit, no Docker invocation)

```bash
file target/smelt-test-cache/assay-linux-aarch64
```

3. **Expected:** `ELF 64-bit LSB executable, ARM aarch64, ... statically linked`

### 4. Full workspace suite unaffected

```bash
cargo test --workspace 2>&1 | grep -E "^test result|FAILED|error\["
```

1. **Expected:** 7 lines of `test result: ok.` — no FAILED, no `error[`

## Edge Cases

### Binary builder graceful skip when assay source unavailable

```bash
# Without ASSAY_SOURCE_DIR and without ../../assay sibling
cargo test -p smelt-cli --test docker_lifecycle test_build_linux_assay_binary_caches -- --nocapture
```

1. **Expected:** Test prints skip message (`"Skipping: assay source not found"`) and exits `ok` — does NOT fail

### Integration test graceful skip when Docker unavailable

Unset or stop the Docker daemon, then:
```bash
cargo test -p smelt-cli --test docker_lifecycle test_real_assay_manifest_parsing -- --nocapture
```

1. **Expected:** Test detects Docker unavailability and returns early with a skip message — does NOT fail with a panic or connection error

### Phase 5.5 failure path — config write fails

This is exercised in the existing error-path logic; to observe: provision a container in which `/workspace` is read-only, then run Phase 5.5. Expected: teardown is triggered and `SmeltError::Provider` is returned with container ID and stderr from the failed exec.

## Failure Signals

- `"No Assay project found"` in assay stderr → Phase 5.5 config write (step 5a) failed or was not reached
- `"unknown field"` in assay stderr → TOML schema mismatch; `deny_unknown_fields` rejected a generated field
- `"ManifestParse"` or `"ManifestValidation"` in assay stderr → TOML parse or semantic validation error
- `test_real_assay_manifest_parsing` fails without printing assay stderr → test skipped due to missing Docker or missing cached binary (check preconditions)
- Phase 5.5 `eprintln!` lines absent from `smelt run` output → Phase 5 (provision) failed before reaching Phase 5.5
- `cargo test --workspace` shows FAILED → a change introduced a regression in the broader test suite

## Requirements Proved By This UAT

No `.kata/REQUIREMENTS.md` exists — operating in legacy compatibility mode. This UAT proves:

- Smelt-generated TOML files (spec files + run manifest) are accepted by the real `assay` binary without `deny_unknown_fields` schema violations
- Phase 5.5 (assay setup: config write, specs dir, per-session spec files) executes in the correct order between container provision and manifest write
- The Linux assay binary build-and-cache mechanism is reliable and skips gracefully when source/Docker unavailable
- All pre-existing tests continue to pass after Phase 5.5 wiring

## Not Proven By This UAT

- Full end-to-end `smelt run` with a real manifest, real Claude API key, and real git operations — assay exits at Phase 2 (git checkout) in the test container; no real coding session runs
- Streaming exec (S03) — output from `assay run` in the integration test is buffered until exec completes, not streamed
- Exit code 2 handling (S04) — gate failures are not exercised in this slice
- Multi-machine or Docker Compose runtimes — only single-container Alpine tested
- Performance under real workloads — test uses a minimal two-session manifest with no actual session content

## Notes for Tester

- The integration test (`test_real_assay_manifest_parsing`) runs against a cached binary; if the cache is stale, delete `target/smelt-test-cache/assay-linux-aarch64` and rebuild with `ASSAY_SOURCE_DIR` set.
- Assay intentionally exits non-zero at Phase 2 (git checkout fails — no real git repo in container) — this is expected behavior and the test does NOT assert exit code 0.
- Running with `--nocapture` is recommended for all docker_lifecycle tests — assay's full stderr is printed unconditionally and gives immediate diagnosis if a parse-phase error appears.
- The `target/smelt-test-cache/` directory is excluded from version control; the cached binary is a build artifact and must be regenerated after `cargo clean`.
