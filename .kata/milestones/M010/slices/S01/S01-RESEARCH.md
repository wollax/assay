# S01: StateBackend trait and CapabilitySet — Research

**Researched:** 2026-03-26
**Domain:** Rust trait design, serde extensibility, zero-trait codebase exception
**Confidence:** HIGH

## Summary

S01 introduces the `StateBackend` trait — the first and only trait in the codebase (D001 exception, documented as D149). The work is pure type/contract design with no orchestrator wiring. Three artifacts live in this slice: the `StateBackend` trait + `CapabilitySet` + `LocalFsBackend` skeleton in `assay-core::state_backend`, and the `StateBackendConfig` enum in `assay-types`. All existing 1466 tests must remain green.

The codebase is clean: no `pub trait` exists anywhere in `crates/`. The trait design needs to be object-safe from day one (`Box<dyn StateBackend + Send + Sync>` is the S02 target). Every method must be sync (D150), return `Result<_, AssayError>`, and carry no generic parameters. `LocalFsBackend` is a skeleton for S01 — methods return `Ok(())` or sensible defaults; real implementation is S02.

The biggest risk is method payload types. The trait methods need stable signatures in S01 so S02 can wire them without changing the API surface. S01 must finalize these signatures even though LocalFsBackend bodies are stubs.

## Recommendation

Define the trait methods with concrete payload types sourced from `assay-types` (the `OrchestratorStatus`, `TeamCheckpoint`, and string-path types already in the codebase). Keep the trait minimal — exactly the 7 methods from the boundary map. Object-safety check must pass (no generics, no `Self` in return position). Use `Arc<dyn StateBackend + Send + Sync>` rather than `Box<dyn StateBackend>` in OrchestratorConfig (S02 concern, but plan for it now) because `OrchestratorConfig` currently derives `Clone` — `Arc` is cheaply cloneable while `Box<dyn>` is not.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Atomic state writes | `persist_state()` in executor.rs (`NamedTempFile` + rename + fsync) | All S01 LocalFsBackend writes should use this pattern, consistent with all other persistence code |
| Checkpoint atomic writes | `save_checkpoint()` in `checkpoint/persistence.rs` | Same pattern — tempfile + rename; LocalFsBackend's `save_checkpoint_summary` should call or mirror this |
| serde(default) backward-compatible field addition | D092 pattern on all existing optional fields | `StateBackendConfig` on `RunManifest` (S02) needs `serde(default, skip_serializing_if = "Option::is_none")` |
| Schema snapshot registration | `inventory::submit! { schema_registry::SchemaEntry {...} }` in assay-types | `StateBackendConfig` needs this same registration for the schema snapshot test |

## Existing Code and Patterns

- `crates/assay-core/src/orchestrate/executor.rs:111` — `pub(crate) fn persist_state(run_dir, status)` — atomic JSON write with NamedTempFile+rename; LocalFsBackend's `push_session_event` will eventually call this (S02)
- `crates/assay-core/src/orchestrate/mesh.rs:225–260` — routing thread polls `outbox/` and moves files to `inbox/`; `send_message` / `poll_inbox` on LocalFsBackend wraps this exact logic (S02)
- `crates/assay-core/src/orchestrate/gossip.rs:52–74` — `persist_knowledge_manifest()` — same atomic write pattern; `annotate_run` on LocalFsBackend calls equivalent logic (S02)
- `crates/assay-core/src/checkpoint/persistence.rs:39` — `save_checkpoint()` — atomic markdown write; `save_checkpoint_summary` wraps this
- `crates/assay-core/src/telemetry.rs` — D143 pattern: scoped tokio runtime inside a module, keeping sync core. Same approach applies if async backends internalize their runtime
- `crates/assay-types/src/manifest.rs:14` — `#[cfg(feature = "orchestrate")]` on `RunManifest` fields — pattern for gating state_backend field behind a feature if needed (S02)
- `crates/assay-types/src/schema_registry.rs` — `inventory::submit! { SchemaEntry { name, generate } }` — use for `StateBackendConfig` schema registration
- `crates/assay-types/tests/schema_snapshots.rs` — pattern for adding `state_backend_config_schema_snapshot()` test

## Constraints

- **D001 zero-trait exception**: `StateBackend` must be the only trait. Document the exception and limit scope tightly — one trait, one flags struct, one config enum.
- **Object-safety required**: `Box<dyn StateBackend + Send + Sync>` is the plan for S02. Every trait method must be object-safe: no generic parameters, no `-> Self`, no unconstrained associated types.
- **D007 sync core**: All trait methods return `Result<_, AssayError>` synchronously. No `async fn`, no `impl Future`.
- **RunManifest has `deny_unknown_fields`**: Confirmed on line 34 of `manifest.rs`. Adding `state_backend: Option<StateBackendConfig>` in S02 **requires** `serde(default, skip_serializing_if = "Option::is_none")` — without `serde(default)`, existing manifests without the field will fail to deserialize. The D092 pattern is mandatory.
- **OrchestratorConfig currently derives Clone**: `Clone` is on `OrchestratorConfig`. Adding `backend: Box<dyn StateBackend>` in S02 would break `Clone` unless the trait also derives it (impossible for dyn) or the field is wrapped in `Arc`. Plan for `Arc<dyn StateBackend + Send + Sync>` now so S02 doesn't need to remove Clone from OrchestratorConfig.
- **StateBackendConfig in assay-types, not assay-core**: Consistent with all persisted/config types. `StateBackend` trait and `LocalFsBackend` go in assay-core (implementation concern).
- **Schema snapshot will change in S02**: When `state_backend` field is added to `RunManifest`, `run-manifest-schema.snap` and `manifest-session-schema.snap` must be updated via `cargo insta review`. S01 adds only `StateBackendConfig` schema (new snap, no existing snap changed).
- **1466 tests must remain green**: Current count from `cargo test --workspace`. S01 adds tests; none should break.

## Common Pitfalls

- **Non-object-safe trait method**: If any method takes `impl Trait`, has a generic parameter, or returns `Self`, the trait cannot be used as `dyn StateBackend`. Write a compile-time check: a function `fn _assert_object_safe(_: Box<dyn StateBackend>) {}` in the module to catch this at compile time.
- **Missing `Send + Sync` bounds on the trait**: Worker threads in `thread::scope` require `Send + Sync`. Declare the trait as `pub trait StateBackend: Send + Sync`. Without these, `Box<dyn StateBackend>` cannot cross thread boundaries.
- **StateBackendConfig feature gating**: `StateBackendConfig` is referenced from `RunManifest.state_backend` which is `#[cfg(feature = "orchestrate")]`. Either gate `StateBackendConfig` behind the same feature, or put it in assay-types without any feature gate (preferred: no feature gate since the type is simple and non-orchestrate users may eventually configure backends directly).
- **Payload types not finalized in S01**: If method signatures are left as `todo!()` or use placeholder types, S02 will need to change the trait API — breaking any downstream implementations (LocalFsBackend). Finalize all method signatures in S01 even if LocalFsBackend bodies are `Ok(())` stubs.
- **Forgetting schema snapshot registration**: `StateBackendConfig` needs `inventory::submit!` + a corresponding test in `schema_snapshots.rs`. Missing this means the schema is unversioned and can drift silently.

## Open Risks

- **Method payload types**: The boundary map names 7 methods. Their argument/return types need to be concrete. The candidates from assay-types are: `OrchestratorStatus` (for `push_session_event`/`read_run_state`), `PathBuf` + message content (for `send_message`/`poll_inbox`), `String` (for `annotate_run` — knowledge manifest path notification), and a checkpoint summary type (for `save_checkpoint_summary`). The exact shape of the "message" payload and the return type of `poll_inbox` need to be designed. A simple `(String, Vec<u8>)` tuple (name, contents) avoids introducing a new type for S01.
- **`serde_json::Value` in Custom variant**: `StateBackendConfig::Custom { name: String, config: serde_json::Value }` — `serde_json` is already in assay-types deps (used by other types). No new dep needed.
- **Trait module location**: `assay_core::state_backend` is the natural home. But since `StateBackend` is `pub`, it needs to be re-exported from `assay_core::lib.rs`. Plan the public re-export path now so S02 imports are obvious.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust trait object design | N/A | No skill needed — standard Rust patterns apply |

## Sources

- `crates/assay-types/src/manifest.rs` — RunManifest structure, `deny_unknown_fields` confirmation, serde feature gating patterns (direct code inspection)
- `crates/assay-core/src/orchestrate/executor.rs` — `persist_state` implementation and `OrchestratorConfig` shape (direct code inspection)
- `crates/assay-core/src/orchestrate/mesh.rs` — routing thread inbox/outbox pattern for `send_message`/`poll_inbox` design (direct code inspection)
- `crates/assay-core/src/orchestrate/gossip.rs` — knowledge manifest persistence for `annotate_run` design (direct code inspection)
- `crates/assay-types/tests/schema_snapshots.rs` — schema snapshot test pattern (direct code inspection)
- `.kata/DECISIONS.md` — D001 (zero-trait), D007 (sync core), D092 (serde default pattern), D149 (StateBackend exception), D150 (sync methods), D151 (Box<dyn> in OrchestratorConfig), D152 (tier split), D153 (StateBackendConfig enum) (direct read)
