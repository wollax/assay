---
estimated_steps: 5
estimated_files: 3
---

# T04: smelt-example external crate (R005 validation)

**Slice:** S05 — smelt-core Library API
**Milestone:** M003

## Description

Creates a standalone Cargo project at `/tmp/smelt-example/` that imports `smelt-core` via a path dependency and calls three key library APIs in `#[test]` functions. This is the integration proof for R005: it demonstrates that `smelt-core` can be embedded without the CLI, that the forge feature compiles and the API is callable from an external crate, and that the public re-exports in `lib.rs` are correct and complete.

The project lives outside the workspace (`/tmp/`) intentionally — this is the strongest proof of external embedding, mirroring how Assay or another tool would depend on `smelt-core`.

Key API shape to verify:
- `GitHubForge::new(token: String) -> Result<Self>` — from the `forge` feature; client construction doesn't make network calls
- `JobManifest::from_str(content: &str, source: &Path) -> Result<Self>` — parse a valid manifest TOML string
- `DockerProvider::new() -> Result<Self>` — constructor may return Err if Docker daemon is absent; test uses `.ok()`, does not assert `.is_ok()`

## Steps

1. **Create directory structure**:
   ```bash
   mkdir -p /tmp/smelt-example/src /tmp/smelt-example/tests
   ```

2. **Write `/tmp/smelt-example/Cargo.toml`**:
   ```toml
   [package]
   name = "smelt-example"
   version = "0.1.0"
   edition = "2024"

   [dependencies]
   smelt-core = { path = "/Users/wollax/Git/personal/smelt/crates/smelt-core", features = ["forge"] }

   [dev-dependencies]
   tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
   ```

3. **Write `/tmp/smelt-example/src/lib.rs`** (minimal — just a comment to satisfy the lib target):
   ```rust
   // smelt-example: external embedding proof for smelt-core
   ```

4. **Write `/tmp/smelt-example/tests/api.rs`** with three tests:

   ```rust
   use std::path::Path;

   #[test]
   fn test_githubforge_builds() {
       let forge = smelt_core::GitHubForge::new("test-token".to_string());
       assert!(forge.is_ok(), "GitHubForge::new should succeed: {:?}", forge.err());
   }

   #[test]
   fn test_jobmanifest_parses_minimal_manifest() {
       let toml = r#"
   [job]
   name = "example-job"
   repo = "."
   base_ref = "main"

   [environment]
   runtime = "docker"
   image = "alpine:3"

   [credentials]
   provider = "anthropic"
   model = "claude-sonnet-4-20250514"

   [credentials.env]
   api_key = "ANTHROPIC_API_KEY"

   [[session]]
   name = "test-session"
   spec = "Run a simple test."

   [merge]
   strategy = "sequential"
   order = ["test-session"]
   ai_resolution = false
   target = "main"
   "#;
       let manifest = smelt_core::JobManifest::from_str(toml, Path::new("test.toml"));
       assert!(manifest.is_ok(), "JobManifest::from_str should parse: {:?}", manifest.err());
       let m = manifest.unwrap();
       assert_eq!(m.job.name, "example-job");
   }

   #[test]
   fn test_docker_provider_new_does_not_panic() {
       // DockerProvider::new() may fail when Docker daemon is absent (CI without Docker).
       // The test proves the API is callable and handles both outcomes gracefully.
       let result = smelt_core::DockerProvider::new();
       // Either outcome is acceptable — we're testing the API surface, not Docker availability.
       let _ = result.ok();
   }
   ```

5. **Run the tests**:
   ```bash
   cd /tmp/smelt-example && cargo test 2>&1
   ```
   All three tests must pass. Confirm the output shows `test result: ok. 3 passed; 0 failed`.

## Must-Haves

- [ ] `/tmp/smelt-example/Cargo.toml` exists with `smelt-core = { path = "...", features = ["forge"] }` as a dependency
- [ ] `/tmp/smelt-example/tests/api.rs` has at least `test_githubforge_builds` and `test_jobmanifest_parses_minimal_manifest`
- [ ] `cd /tmp/smelt-example && cargo test` exits 0
- [ ] `test result: ok. 3 passed; 0 failed` (or more if additional tests were added)
- [ ] The project is NOT added to the workspace `Cargo.toml` members list

## Verification

```bash
cd /tmp/smelt-example && cargo test 2>&1 | tail -5
# expected:
# test test_githubforge_builds ... ok
# test test_jobmanifest_parses_minimal_manifest ... ok
# test test_docker_provider_new_does_not_panic ... ok
# test result: ok. 3 passed; 0 failed; ...

# Confirm not in workspace
grep "smelt-example" /Users/wollax/Git/personal/smelt/Cargo.toml
# expected: (no output)
```

## Observability Impact

- Signals added/changed: None at runtime — smelt-example is a test artifact, not deployed code
- How a future agent inspects this: `cd /tmp/smelt-example && cargo test` — authoritative proof that the library embedding story works; `cat /tmp/smelt-example/Cargo.toml` to see the path dep
- Failure state exposed: compiler errors on `cargo test` in /tmp/smelt-example indicate a broken public API (missing re-exports, visibility issues); test failures indicate a broken constructor or parse path

## Inputs

- T03 completed — `#![deny(missing_docs)]` active; `cargo doc` clean; workspace tests green
- `crates/smelt-core/src/lib.rs` — public re-exports (GitHubForge behind cfg feature, DockerProvider, JobManifest all re-exported)
- `examples/job-manifest-forge.toml` — reference for valid manifest TOML structure

## Expected Output

- `/tmp/smelt-example/Cargo.toml` — standalone Cargo project with path dep to smelt-core
- `/tmp/smelt-example/src/lib.rs` — placeholder lib target
- `/tmp/smelt-example/tests/api.rs` — three tests proving GitHubForge, JobManifest, DockerProvider are callable from external crates
