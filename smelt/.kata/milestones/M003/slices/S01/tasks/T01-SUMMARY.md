---
id: T01
parent: S01
milestone: M003
provides:
  - "`forge` feature flag in smelt-core gating octocrab as an optional dependency"
  - "`ForgeConfig`, `PrHandle`, `PrState`, `CiStatus`, `PrStatus` types (unconditional, no octocrab dep)"
  - "`ForgeClient` trait with `create_pr` and `poll_pr_status` using RPITIT async (unconditional)"
  - "`GitHubForge` struct and stub `ForgeClient` impl behind `#[cfg(feature = \"forge\")]`"
  - "`SmeltError::Forge` variant with `forge()` and `forge_with_source()` constructors"
  - "6 failing test skeletons in `forge.rs` — compile clean, panic with `unimplemented!(\"T02\")` / `unimplemented!(\"T03\")`"
key_files:
  - crates/smelt-core/Cargo.toml
  - crates/smelt-core/src/forge.rs
  - crates/smelt-core/src/error.rs
  - crates/smelt-core/src/lib.rs
key_decisions:
  - "D053: octocrab::Error wrapped by stringify initially — confirmed consistent with existing `SmeltError::Provider` stringification strategy; T02 must evaluate if a typed source field is feasible"
  - "D055: ForgeConfig and trait types unconditional; only GitHubForge is feature-gated — allows S02 manifest.rs to use ForgeConfig without enabling forge feature"
patterns_established:
  - "`forge_for_server()` test helper in forge.rs — constructs GitHubForge pointed at a MockServer by direct field construction, bypassing new()"
  - "wiremock + octocrab test pattern: `MockServer::start()` → `OctocrabBuilder::base_uri(server.uri()).unwrap()` → `GitHubForge { client }` → mount mocks → call method"
observability_surfaces:
  - "`SmeltError::Forge { operation, message }` — operation field tags the failing API call (\"create_pr\", \"poll_pr_status\", \"init\"); message carries octocrab error string or formatted source"
  - "`cargo test -p smelt-core --features forge -- --nocapture` — wiremock request logs + panic messages with T02/T03 labels surface which task is not yet implemented"
duration: 30min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T01: Feature flag, types, ForgeClient trait, and failing test skeleton

**`forge` feature flag, all public types, `ForgeClient` trait, `GitHubForge` stub, `SmeltError::Forge`, and 6 wiremock-wired test skeletons that compile and panic at `unimplemented!("T02")`/`unimplemented!("T03")`.**

## What Happened

Added `octocrab = { version = "0.49", optional = true }` to `smelt-core/Cargo.toml` under `[features] forge = ["dep:octocrab"]`, with `wiremock = "0.6"` and `serde_json = "1"` as unconditional dev-dependencies (tests must compile even without `--features forge`).

Created `crates/smelt-core/src/forge.rs` with all five public types (`ForgeConfig`, `PrHandle`, `PrState`, `CiStatus`, `PrStatus`) and the `ForgeClient` trait defined unconditionally — no octocrab dependency. `ForgeConfig` carries `#[serde(deny_unknown_fields)]` consistent with D017. `ForgeClient` uses RPITIT (`impl Future<Output = ...> + Send`) matching the `RuntimeProvider` trait shape (D019).

`GitHubForge`, its `new()` constructor, and its `ForgeClient` impl are all under `#[cfg(feature = "forge")]`. The stub impl methods call `unimplemented!("T02")` and `unimplemented!("T03")` to give downstream tasks unambiguous compilation targets.

Added `SmeltError::Forge { operation: String, message: String }` to `error.rs` with two constructors: `forge()` and `forge_with_source()` (which stringifies the source into the message). The `forge_with_source()` design defers the `octocrab::Error: Send + Sync + 'static` bounds question to T02 (D053).

Wired `pub mod forge` into `lib.rs` with unconditional re-exports for types and trait, and `#[cfg(feature = "forge")]` re-export for `GitHubForge`.

Added a `forge_for_server()` helper in the test module that constructs `GitHubForge { client }` directly (field access) to redirect octocrab at a `MockServer` URI. The 6 test functions each start a mock server, mount a wiremock `Mock`, and call the stub method — they all panic as expected.

## Verification

```
# No-forge build (zero compile errors)
cargo build -p smelt-core 2>&1 | grep -E "^error" | wc -l  → 0

# Forge build (zero compile errors, one dead_code warning on `client` field — expected)
cargo build -p smelt-core --features forge 2>&1 | grep -E "^error" | wc -l  → 0

# octocrab absent without feature
cargo tree -p smelt-core 2>/dev/null | grep octocrab  → (no output)

# octocrab present with feature
cargo tree -p smelt-core --features forge 2>/dev/null | grep octocrab  → "├── octocrab v0.49.5"

# 6 tests fail at runtime (not compile time) with unimplemented! panics
cargo test -p smelt-core --features forge 2>&1 | grep "FAILED" | wc -l  → 7 (6 tests + summary line)
cargo test -p smelt-core --features forge 2>&1 | grep "not implemented" | wc -l  → 6
# 112 pre-existing tests still pass
```

## Diagnostics

- `SmeltError::Forge { operation, message }` — stable match arm for all forge failures; `operation` field identifies which API call failed
- `cargo test -p smelt-core --features forge -- --nocapture` shows wiremock request matching logs and panic messages with "T02"/"T03" labels
- `unimplemented!("T02")` / `unimplemented!("T03")` in panic output unambiguously identify which task needs to implement each method

## Deviations

- Added `serde_json = "1"` to dev-dependencies (not mentioned in task plan) — required for `serde_json::json!` in wiremock `set_body_json()` calls. Octocrab pulls it in as a transitive dep but direct declaration is needed for the `json!` macro in test code.
- Test assertions are commented out (not active assertions) since the methods are `unimplemented!()`. The assertions are in comments showing what T02/T03 must make pass, keeping the test bodies meaningful as executable documentation.

## Known Issues

- One `dead_code` warning on the `client` field of `GitHubForge` — expected since impl bodies are `unimplemented!()`. Will resolve when T02/T03 implement the method bodies.

## Files Created/Modified

- `crates/smelt-core/Cargo.toml` — forge feature flag, octocrab optional dep, wiremock + serde_json dev-deps
- `crates/smelt-core/src/forge.rs` — new file: types, ForgeClient trait, GitHubForge stub, 6 test skeletons
- `crates/smelt-core/src/error.rs` — SmeltError::Forge variant + forge() and forge_with_source() constructors
- `crates/smelt-core/src/lib.rs` — forge module declaration + pub use re-exports
