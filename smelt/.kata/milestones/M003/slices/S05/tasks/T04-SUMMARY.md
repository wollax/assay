---
id: T04
parent: S05
milestone: M003
provides:
  - /tmp/smelt-example/Cargo.toml — standalone Cargo project outside the workspace importing smelt-core via path dep with the forge feature
  - /tmp/smelt-example/tests/api.rs — three integration tests: GitHubForge::new (async/tokio), JobManifest::from_str, DockerProvider::new
  - Confirmed GitHubForge::new requires a Tokio runtime (tower::buffer initialises on construction)
  - Confirmed [[session]] blocks require both harness and timeout fields in manifest TOML
  - R005 validation: 3/3 tests pass; embedding story is end-to-end proven
key_files:
  - /tmp/smelt-example/Cargo.toml
  - /tmp/smelt-example/src/lib.rs
  - /tmp/smelt-example/tests/api.rs
key_decisions:
  - "GitHubForge::new must be called from within a Tokio runtime — test uses #[tokio::test] not #[test]"
  - "[[session]] TOML blocks require both harness (String) and timeout (u64) fields — the reference example at examples/job-manifest-forge.toml has both"
patterns_established:
  - "External embedding tests for smelt-core live in /tmp/smelt-example/ outside the workspace and are run with 'cd /tmp/smelt-example && cargo test'"
observability_surfaces:
  - "cd /tmp/smelt-example && cargo test — authoritative external embedding proof; compiler errors indicate missing re-exports or visibility regressions; test failures indicate broken constructors or parse paths"
duration: 10min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T04: smelt-example external crate (R005 validation)

**Standalone `smelt-example` crate at `/tmp/smelt-example/` imports `smelt-core` via path dependency with the forge feature and passes all three integration tests — `GitHubForge::new`, `JobManifest::from_str`, and `DockerProvider::new` — confirming R005.**

## What Happened

Created a three-file Cargo project at `/tmp/smelt-example/` outside the workspace to mirror how an external consumer would depend on `smelt-core`. The project uses a path dependency with `features = ["forge"]`.

Two issues were discovered and fixed during test execution:

1. **`GitHubForge::new` requires a Tokio runtime** — the `tower::buffer` component initialises on construction and panics outside a runtime. Fixed by switching the test to `#[tokio::test]`.

2. **`[[session]]` TOML blocks require both `harness` and `timeout` fields** — the initial TOML omitted these, causing TOML parse errors. Fixed by adding `harness = "cargo test"` and `timeout = 300` to match the reference manifest at `examples/job-manifest-forge.toml`.

After both fixes all three tests passed on the second compile attempt.

## Verification

```
cd /tmp/smelt-example && cargo test
# running 3 tests
# test test_docker_provider_new_does_not_panic ... ok
# test test_jobmanifest_parses_minimal_manifest ... ok
# test test_githubforge_builds ... ok
# test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
```

Full slice verification also confirmed clean:
- `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps` → 0 errors
- `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps --features forge` → 0 errors
- `cargo test --workspace -q | grep "failed"` → no failures (197 tests total)
- `grep "smelt-example" Cargo.toml` → no output (not added to workspace)

## Diagnostics

`cd /tmp/smelt-example && cargo test` — re-runnable proof of embedding story. Compiler errors signal missing re-exports or visibility regressions in smelt-core. Test failures signal broken API constructors or parse paths.

## Deviations

The task plan's `[[session]]` TOML did not include `harness` or `timeout` fields (both required). Added them by consulting the reference example. The task plan's `test_githubforge_builds` used `#[test]` but `GitHubForge::new` requires a Tokio runtime; changed to `#[tokio::test] async fn`.

## Known Issues

None.

## Files Created/Modified

- `/tmp/smelt-example/Cargo.toml` — standalone Cargo project with smelt-core path dep (forge feature) and tokio dev-dep
- `/tmp/smelt-example/src/lib.rs` — placeholder lib target (one comment line)
- `/tmp/smelt-example/tests/api.rs` — three integration tests validating GitHubForge, JobManifest, and DockerProvider from an external crate
