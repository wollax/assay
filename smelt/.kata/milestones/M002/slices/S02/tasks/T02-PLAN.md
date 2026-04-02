---
estimated_steps: 5
estimated_files: 1
---

# T02: Linux assay binary builder and container injection helper

**Slice:** S02 — Real Assay Binary + Production Wiring
**Milestone:** M002

## Description

The integration test (`test_real_assay_manifest_parsing`, T03) requires a real `assay` binary that executes inside a Linux aarch64 Alpine container. The macOS Mach-O binary at `assay/target/debug/assay` cannot run there (exec format error). This task adds two helper functions to `docker_lifecycle.rs`:

1. `build_linux_assay_binary()` — builds the assay binary inside a `rust:alpine` Docker container and caches it at `target/smelt-test-cache/assay-linux-aarch64`. Returns `None` if the assay source directory can't be found or the Docker build fails (enabling graceful skip in tests).
2. `inject_binary_to_container()` — copies a host file into a running container using `docker cp` (subprocess), avoiding the base64 exec approach which is too slow/unreliable for 50–100 MB debug binaries.

A smoke test `test_build_linux_assay_binary_caches` verifies the builder by running it and checking that the cache file exists with non-zero size (skips if assay source or Docker unavailable).

## Steps

1. **Add `build_linux_assay_binary()` helper**: At the top of `docker_lifecycle.rs` (after the existing helpers), add a function that:
   - Detects assay source directory: first checks `std::env::var("ASSAY_SOURCE_DIR")`, then checks `<workspace_root>/../../assay` (sibling repo pattern for local dev — `workspace_root()` is `CARGO_MANIFEST_DIR/../..`); returns `None` if neither exists.
   - Resolves cache path: `<workspace_root>/target/smelt-test-cache/assay-linux-aarch64`; if file exists, returns `Some(cache_path)` immediately (skip rebuild).
   - Creates `target/smelt-test-cache/` directory via `std::fs::create_dir_all`.
   - Runs the Docker build via `std::process::Command::new("docker")` with args: `["run", "--rm", "--platform", "linux/arm64", "-v", "<assay_src>:/assay:ro", "-v", "<cargo_home>/registry:/usr/local/cargo/registry", "-v", "<cache_dir_build>:/build", "-e", "CARGO_TARGET_DIR=/build", "-w", "/assay", "rust:alpine", "sh", "-c", "cargo build --bin assay 2>&1"]` where `<cache_dir_build>` is `target/smelt-test-cache/assay-build` (separate dir to avoid mixing macOS artifacts); `<cargo_home>` is `$HOME/.cargo` (fallback to `$HOME/.cargo` if `CARGO_HOME` not set).
   - On Docker command failure (non-zero exit), logs the output to stderr and returns `None`.
   - On success, copies `target/smelt-test-cache/assay-build/debug/assay` to `target/smelt-test-cache/assay-linux-aarch64` via `std::fs::copy`; returns `Some(cache_path)`.
   - Note: the `--bin assay` flag matches the `[[bin]] name = "assay"` entry in `assay-cli/Cargo.toml`; the cargo target dir is mounted as a volume so the Linux build artifacts don't contaminate the macOS `target/` tree.

2. **Add `inject_binary_to_container()` helper**: Simple function with signature `fn inject_binary_to_container(container_id: &str, host_path: &std::path::Path, dest_path: &str) -> bool` that runs `docker cp <host_path> <container_id>:<dest_path>` via subprocess; returns `true` iff exit status is success.

3. **Add smoke test `test_build_linux_assay_binary_caches`**: `#[tokio::test]` that calls `build_linux_assay_binary()`; if `None`, returns (skip); if `Some(path)`, asserts `path.exists()` and `path.metadata().unwrap().len() > 0`; also verifies the cache path ends with `assay-linux-aarch64`. This test validates the builder end-to-end without needing a live container.

4. **Handle the cargo registry volume path**: Use `std::env::var("CARGO_HOME").unwrap_or_else(|_| format!("{}/.cargo", std::env::var("HOME").unwrap()))` to find the registry. Pass as an absolute path to Docker `-v` arg to enable crate download caching across runs.

5. **Verify compilation**: Run `cargo build -p smelt-cli` to confirm the helper functions compile without errors before moving to T03.

## Must-Haves

- [ ] `build_linux_assay_binary()` returns `None` when `ASSAY_SOURCE_DIR` is not set AND `../../assay` sibling doesn't exist — test skips gracefully
- [ ] Cache hit detection: if `target/smelt-test-cache/assay-linux-aarch64` exists, the function returns it without re-running Docker build
- [ ] The Docker build uses `CARGO_TARGET_DIR=/build` mounted as a separate volume, so Linux artifacts never appear in the macOS `target/` of the assay source
- [ ] `inject_binary_to_container()` uses subprocess `docker cp` — no bollard API needed
- [ ] `test_build_linux_assay_binary_caches` compiles and either passes (binary cached) or skips (source/Docker unavailable), never fails with an unexpected panic

## Verification

```bash
# Confirm helpers compile
cargo build -p smelt-cli 2>&1 | grep -E "error\[|warning\[E"

# Run the smoke test (skips if Docker/assay source not found)
cargo test -p smelt-cli --test docker_lifecycle test_build_linux_assay_binary_caches -- --nocapture

# If assay source available, confirm binary was cached
ls -lh target/smelt-test-cache/assay-linux-aarch64

# Confirm it's a Linux ELF binary (not macOS Mach-O)
file target/smelt-test-cache/assay-linux-aarch64
# Expected: ELF 64-bit LSB executable, ARM aarch64, ...
```

## Observability Impact

- Signals added/changed: Docker build stdout/stderr printed to test stderr on build failure via `eprintln!`; "Skipping: assay source not found" printed when source is absent; cache hit is silent (fast path)
- How a future agent inspects this: run the smoke test with `--nocapture` to see Docker build output; check `target/smelt-test-cache/` for cached binary; `file <binary>` confirms Linux ELF
- Failure state exposed: Docker build failures print full cargo output to test stderr; the helper returns `None` rather than panicking, so test consumers see a skip not a crash

## Inputs

- `crates/smelt-cli/tests/docker_lifecycle.rs` — existing file where helpers are added; `workspace_root()` function already exists and returns `CARGO_MANIFEST_DIR/../..`
- `/Users/wollax/Git/personal/assay/crates/assay-cli/Cargo.toml` — confirms `[[bin]] name = "assay"` so the correct `--bin assay` flag is used
- Research constraint: "No Linux Rust targets installed" — must build via Docker; "Docker build cache" — use registry volume + separate build target dir; "Binary size" — use `docker cp` not base64 exec for injection

## Expected Output

- `crates/smelt-cli/tests/docker_lifecycle.rs` — two new helper functions (`build_linux_assay_binary`, `inject_binary_to_container`) and one smoke test (`test_build_linux_assay_binary_caches`)
- `target/smelt-test-cache/assay-linux-aarch64` — cached Linux aarch64 ELF binary (if assay source + Docker available)
- `cargo build -p smelt-cli` exits 0
