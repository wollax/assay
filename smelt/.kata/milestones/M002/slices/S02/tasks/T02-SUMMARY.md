---
id: T02
parent: S02
milestone: M002
provides:
  - build_linux_assay_binary() helper in docker_lifecycle.rs — builds assay ELF binary for Linux aarch64 inside rust:alpine container and caches at target/smelt-test-cache/assay-linux-aarch64
  - inject_binary_to_container() helper in docker_lifecycle.rs — injects host file into running container via docker cp subprocess
  - test_build_linux_assay_binary_caches smoke test — verifies builder end-to-end or skips gracefully
  - workspace_root() helper in docker_lifecycle.rs — returns workspace root from CARGO_MANIFEST_DIR
key_files:
  - crates/smelt-cli/tests/docker_lifecycle.rs
key_decisions:
  - apk add --no-cache musl-dev is required inside rust:alpine before cargo build (musl libc dev headers needed for statically linked binaries); added to the docker run sh -c command
  - inject_binary_to_container() annotated with #[allow(dead_code)] to avoid compiler warnings until T03 consumes it
  - workspace_root() added since it was not present in the file despite being referenced in the task plan as "already exists"
patterns_established:
  - build_linux_assay_binary() follows the cache-first pattern — if target/smelt-test-cache/assay-linux-aarch64 exists, return immediately without Docker invocation
  - Helper functions that are only consumed by future tasks use #[allow(dead_code)] to prevent spurious warnings at the compilation stage
  - ASSAY_SOURCE_DIR env var overrides sibling-repo detection for CI or non-standard directory layouts
observability_surfaces:
  - eprintln! on Docker build failure shows full cargo build output to test stderr
  - "Skipping: assay source not found at <path>" on missing source
  - "Building Linux aarch64 assay binary from <path> ..." on build start
  - "Cached Linux aarch64 assay binary at <path>" on successful cache write
  - Cache hit is silent (fast path, no output)
  - ASSAY_SOURCE_DIR env var documented in skip message so future agents know the override
duration: ~45 minutes (including Docker build of 130MB ELF binary)
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T02: Linux assay binary builder and container injection helper

**Added `build_linux_assay_binary()` and `inject_binary_to_container()` helpers to `docker_lifecycle.rs`, producing a cached 130MB Linux aarch64 ELF assay binary at `target/smelt-test-cache/assay-linux-aarch64`.**

## What Happened

Added three new items to `crates/smelt-cli/tests/docker_lifecycle.rs`:

1. **`workspace_root()`** — returns `CARGO_MANIFEST_DIR/../..` canonicalized. This was listed as "already exists" in the task plan but was absent from the file; added it as a prerequisite.

2. **`build_linux_assay_binary()`** — detects the assay source directory (via `ASSAY_SOURCE_DIR` env var or sibling-repo fallback), checks for a cached binary at `target/smelt-test-cache/assay-linux-aarch64`, and on cache miss runs a `docker run` command with `rust:alpine` + platform `linux/arm64` to build the binary. Uses three volume mounts: assay source (`:ro`), cargo registry cache, and a separate build target dir to prevent contaminating the macOS `target/`. Returns `None` gracefully on any failure.

   One deviation from the plan: the Docker build command needed `apk add --no-cache musl-dev &&` prepended to the `cargo build` invocation because `rust:alpine` doesn't ship musl libc dev headers by default, causing linker failures. This was discovered during the Docker build and fixed.

3. **`inject_binary_to_container()`** — runs `docker cp <host_path> <container_id>:<dest_path>` via `std::process::Command`. Annotated `#[allow(dead_code)]` since T03 is its first consumer.

4. **`test_build_linux_assay_binary_caches`** — tokio smoke test. Calls `build_linux_assay_binary()`: if `None`, prints skip message and returns. If `Some(path)`, asserts existence, non-zero size, and correct filename suffix. Also verifies the cache-hit fast path by calling the function a second time and checking it returns the same path.

## Verification

```
# Compile check — no errors
cargo build -p smelt-cli 2>&1 | grep -E "^error|warning\[E"
# (no output — clean build)

# Smoke test — graceful skip without ASSAY_SOURCE_DIR
cargo test -p smelt-cli --test docker_lifecycle test_build_linux_assay_binary_caches -- --nocapture
# test result: ok. 1 passed

# Smoke test — full build with ASSAY_SOURCE_DIR set
ASSAY_SOURCE_DIR=/Users/wollax/Git/personal/assay cargo test -p smelt-cli --test docker_lifecycle test_build_linux_assay_binary_caches -- --nocapture
# "Cached Linux aarch64 assay binary at .../target/smelt-test-cache/assay-linux-aarch64"
# test result: ok. 1 passed (42.95s)

# Confirm Linux ELF
file target/smelt-test-cache/assay-linux-aarch64
# ELF 64-bit LSB executable, ARM aarch64, version 1 (SYSV), statically linked, ...
# Size: 130MB

# Full workspace — all tests pass
cargo test --workspace
# 7 test suites: all "test result: ok." — no FAILED, no error[]
```

## Diagnostics

- Run `ASSAY_SOURCE_DIR=<path> cargo test -p smelt-cli --test docker_lifecycle test_build_linux_assay_binary_caches -- --nocapture` to trigger a real Docker build and see full cargo output.
- Check `target/smelt-test-cache/` for the cached binary; `file target/smelt-test-cache/assay-linux-aarch64` confirms Linux ELF aarch64.
- Docker build failures print full stdout/stderr to test stderr before returning `None`.
- Cache hit is silent — if the function completes instantly with `Some(path)`, that's the fast path.

## Deviations

- **`workspace_root()` was absent** from `docker_lifecycle.rs` despite being listed as "already exists" in the task plan. Added it as a plain free function.
- **`apk add --no-cache musl-dev` prepended** to the Docker `sh -c` command. The task plan's Docker command was `"cargo build --bin assay 2>&1"` but `rust:alpine` lacks musl-dev headers needed for static linking. The fix: `"apk add --no-cache musl-dev && cargo build --bin assay 2>&1"`.

## Known Issues

- The `--bin assay` flag is not needed for the current assay workspace (the default build produces the binary), but it's present as specified for explicitness.
- Cache lives at `target/smelt-test-cache/` which is `.gitignore`d by Cargo's default ignore rules — this is expected behavior; the binary is an artifact, not source.

## Files Created/Modified

- `crates/smelt-cli/tests/docker_lifecycle.rs` — added `workspace_root()`, `build_linux_assay_binary()`, `inject_binary_to_container()`, and `test_build_linux_assay_binary_caches`
