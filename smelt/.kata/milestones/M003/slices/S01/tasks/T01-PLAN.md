---
estimated_steps: 5
estimated_files: 4
---

# T01: Feature flag, types, ForgeClient trait, and failing test skeleton

**Slice:** S01 — GitHub Forge Client
**Milestone:** M003

## Description

Establish the `forge` feature flag in `smelt-core`, define all public types (`ForgeConfig`, `PrHandle`, `PrState`, `CiStatus`, `PrStatus`), define the `ForgeClient` trait, stub `GitHubForge` with `unimplemented!()` bodies, add the `SmeltError::Forge` variant, wire the module into `lib.rs`, and write all 6 unit tests. Tests must compile but fail at runtime (due to `unimplemented!()`). This gives T02 and T03 unambiguous targets to make green.

## Steps

1. **Cargo.toml**: Add `octocrab = { version = "0.49", optional = true }` to `[dependencies]` in `crates/smelt-core/Cargo.toml`; add `[features] forge = ["dep:octocrab"]`; add `wiremock = "0.6"` to `[dev-dependencies]` (unconditional — dev tests always need it so they can compile even without `--features forge`). Do NOT add octocrab to workspace `Cargo.toml` — it stays local to `smelt-core`.

2. **error.rs**: Add `SmeltError::Forge { operation: String, message: String }` variant with `#[error("forge {operation} failed: {message}")]`. Add two constructors:
   ```rust
   pub fn forge(operation: impl Into<String>, message: impl Into<String>) -> Self {
       Self::Forge { operation: operation.into(), message: message.into() }
   }
   pub fn forge_with_source(
       operation: impl Into<String>,
       message: impl Into<String>,
       source: impl std::error::Error + Send + Sync + 'static,
   ) -> Self {
       // Store source in message for now: format!("{message}: {source}") — upgrade to a source field if needed after checking octocrab::Error bounds
       Self::Forge { operation: operation.into(), message: format!("{}: {source}", message.into()) }
   }
   ```
   Note: If octocrab::Error implements `Send + Sync + 'static`, add a `source` field like the `Provider` variant; otherwise stringify. Decide during T02 implementation and record as a decision.

3. **forge.rs** — Create `crates/smelt-core/src/forge.rs`:
   ```
   // Types
   ForgeConfig { provider: String, repo: String, token_env: String }  // #[derive(Debug, Deserialize, Clone)] #[serde(deny_unknown_fields)]
   PrHandle { url: String, number: u64 }                               // #[derive(Debug, Clone, PartialEq)]
   PrState { Open, Merged, Closed }                                    // #[derive(Debug, Clone, PartialEq)]
   CiStatus { Pending, Passing, Failing, Unknown }                     // #[derive(Debug, Clone, PartialEq)]
   PrStatus { state: PrState, ci_status: CiStatus, review_count: u32 } // #[derive(Debug, Clone)]
   
   // Trait (uses #[cfg(feature = "forge")] gating)
   pub trait ForgeClient {
       async fn create_pr(&self, repo: &str, head: &str, base: &str, title: &str, body: &str)
           -> crate::Result<PrHandle>;
       async fn poll_pr_status(&self, repo: &str, number: u64) -> crate::Result<PrStatus>;
   }
   
   // Struct
   #[cfg(feature = "forge")]
   pub struct GitHubForge { client: octocrab::Octocrab }
   
   #[cfg(feature = "forge")]
   impl GitHubForge {
       pub fn new(token: String) -> crate::Result<Self> { ... octocrab builder ... }
   }
   
   #[cfg(feature = "forge")]
   impl ForgeClient for GitHubForge {
       async fn create_pr(...) -> ... { unimplemented!("T02") }
       async fn poll_pr_status(...) -> ... { unimplemented!("T03") }
   }
   ```
   Use `#[cfg(feature = "forge")]` on anything that imports from octocrab. The types (`ForgeConfig`, `PrHandle`, `PrState`, `CiStatus`, `PrStatus`) and the `ForgeClient` trait can be defined unconditionally (they don't depend on octocrab) — this allows S02 to reference `ForgeConfig` in manifest.rs without needing the forge feature. `GitHubForge` and its impl are `#[cfg(feature = "forge")]` only.

4. **lib.rs**: Add at the top of the module list:
   ```rust
   pub mod forge;
   ```
   And in the pub use block:
   ```rust
   #[cfg(feature = "forge")]
   pub use forge::{GitHubForge};
   pub use forge::{ForgeClient, ForgeConfig, PrHandle, PrState, CiStatus, PrStatus};
   ```
   The types and trait are always re-exported; only `GitHubForge` is feature-gated.

5. **Test skeleton**: Add `#[cfg(test)]` module in `forge.rs` with all 6 test functions using `#[tokio::test]`. Each test must:
   - Start a `MockServer::start().await`
   - Use `octocrab::OctocrabBuilder::new().base_uri(server.uri())?.personal_token("test-token").build()?` to get a redirected client (note: `base_uri` returns `Result` in 0.49 — handle `?`)
   - Construct `GitHubForge { client }` directly (field access, not `new()`) to bypass token-at-construction
   - Mount a `Mock::given(...).respond_with(...)` on the server
   - Call the method and assert the result
   - The `create_pr` tests will panic at `unimplemented!("T02")`; `poll_pr_status` tests at `unimplemented!("T03")` — this is correct and expected
   
   Test function signatures (all `#[tokio::test]` + `#[cfg(feature = "forge")]`):
   - `async fn test_create_pr_happy_path()`
   - `async fn test_create_pr_auth_error()`
   - `async fn test_create_pr_validation_error()`
   - `async fn test_poll_pr_status_open_pending()`
   - `async fn test_poll_pr_status_merged_passing()`
   - `async fn test_poll_pr_status_closed_failing()`

## Must-Haves

- [ ] `cargo build -p smelt-core` compiles with zero errors (no forge feature)
- [ ] `cargo build -p smelt-core --features forge` compiles with zero errors (with forge feature, stubs are valid)
- [ ] `cargo tree -p smelt-core | grep octocrab` prints nothing (zero deps without feature)
- [ ] `cargo tree -p smelt-core --features forge | grep octocrab` shows octocrab in the tree
- [ ] All 6 test functions exist in `forge.rs` under `#[cfg(test)]` and `#[cfg(feature = "forge")]`
- [ ] Running `cargo test -p smelt-core --features forge` fails with panics (not compile errors) — exactly 6 test failures all reading `not yet implemented`
- [ ] `SmeltError::Forge` variant exists with `forge()` and `forge_with_source()` constructors

## Verification

```bash
# No-forge build
cargo build -p smelt-core 2>&1 | grep -E "^error" | wc -l  # must be 0

# Forge build with stubs
cargo build -p smelt-core --features forge 2>&1 | grep -E "^error" | wc -l  # must be 0

# octocrab absent from no-forge tree
cargo tree -p smelt-core 2>/dev/null | grep octocrab  # no output

# octocrab present in forge tree
cargo tree -p smelt-core --features forge 2>/dev/null | grep octocrab  # shows octocrab

# Tests compile and fail at runtime (not compile time)
cargo test -p smelt-core --features forge 2>&1 | grep "panicked\|not yet implemented" | wc -l  # ≥ 6
cargo test -p smelt-core --features forge 2>&1 | grep "FAILED" | wc -l  # should be 6
```

## Observability Impact

- Signals added/changed: `SmeltError::Forge` variant — any forge operation failure carries `operation` and `message` fields, giving callers a stable match arm and clear error context
- How a future agent inspects this: `cargo test -p smelt-core --features forge -- --nocapture` surfaces wiremock request logs and panic messages
- Failure state exposed: `unimplemented!("T02")` / `unimplemented!("T03")` panic messages explicitly name which task needs to implement them — unambiguous progress tracking

## Inputs

- `crates/smelt-core/src/error.rs` — Provider variant structure to mirror
- `crates/smelt-core/src/provider.rs` — RPITIT trait shape to follow for ForgeClient
- `crates/smelt-core/src/lib.rs` — Existing pub use pattern to extend
- Research notes: octocrab 0.49 `OctocrabBuilder::base_uri()` returns `Result<Self>` — handle `?`

## Expected Output

- `crates/smelt-core/Cargo.toml` — forge feature flag + octocrab optional dep + wiremock dev-dep
- `crates/smelt-core/src/forge.rs` — types, trait, GitHubForge stub, 6 test skeletons
- `crates/smelt-core/src/error.rs` — SmeltError::Forge variant + constructors
- `crates/smelt-core/src/lib.rs` — forge module wired in
