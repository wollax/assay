# S02: Harness Crate & Profile Type — Research

**Date:** 2026-03-16

## Summary

S02 delivers two things: a new `assay-harness` leaf crate in the workspace and a `HarnessProfile` type in `assay-types`. The crate is scaffolding — it needs to exist, compile, and have the right dependency edges. The type is the interesting part: it must capture a complete agent configuration (prompt layers, settings overrides, hook definitions) in a serializable DTO that downstream slices S03, S04, S06, and S07 all consume.

The codebase has well-established patterns for new types in `assay-types` (serde + schemars derives, `deny_unknown_fields`, `inventory::submit!` for schema registry, insta snapshot tests) and for crate structure (workspace deps in root `Cargo.toml`, thin `lib.rs` with module stubs). The main design risk is getting the `HarnessProfile` shape right so S03–S04 don't need to rework it. The boundary map in the roadmap is explicit about what S02 must produce.

This is a low-execution-risk slice — mostly type design and workspace plumbing. The primary value is getting the type contract right for downstream consumers.

## Recommendation

**Create `crates/assay-harness` as a minimal leaf crate** with `lib.rs` containing module stubs (`prompt`, `settings`, `claude` — matching what S03/S04 will fill in). Add it to workspace members. It depends on `assay-core` and `assay-types` per D003.

**Design `HarnessProfile` in `assay-types/src/harness.rs`** with these sub-types:
- `PromptLayer` — ordered layers for prompt assembly (role/priority enum + content string)
- `SettingsOverride` — key-value settings the harness should apply (model, permissions, tool access)
- `HookContract` — lifecycle event definitions (event type + handler config)
- `HarnessProfile` — the top-level composite referencing the above

All types get `Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Eq`, `#[serde(deny_unknown_fields)]`, `inventory::submit!`, and insta schema snapshot tests. Follow the exact pattern established by `GateEvalContext`, `WorkSession`, `FeatureSpec`, etc.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Schema generation | `schemars` (already in workspace) | All types use it; schema registry pattern is established |
| Schema snapshot testing | `insta` (already in workspace) | Every schema type has a snapshot test in `schema_snapshots.rs` |
| Atomic ID for schema registry | `inventory` crate (already in workspace) | Decentralized registration pattern used by all types |

## Existing Code and Patterns

- `crates/assay-types/src/session.rs` — `GateEvalContext` is the most recent type added (S01). Copy its derive pattern, `deny_unknown_fields`, `inventory::submit!`, and test structure.
- `crates/assay-types/src/feature_spec.rs` — Most complex type in the codebase (~400 lines). Shows how to compose nested structs with `deny_unknown_fields` at every level, optional fields with `skip_serializing_if`, and enums with `rename_all = "kebab-case"`.
- `crates/assay-types/src/work_session.rs` — `WorkSession` with `SessionPhase` enum shows the lifecycle state machine pattern. `HookEvent` may follow a similar enum pattern.
- `crates/assay-types/src/schema_registry.rs` — Schema registry with `inventory::collect!`. Every new type must call `inventory::submit!` with a kebab-case name.
- `crates/assay-types/tests/schema_snapshots.rs` — Every schema-registered type has a corresponding snapshot test. New types need entries here.
- `crates/assay-types/src/lib.rs` — Module declaration + `pub use` re-exports. New `harness` module and types must be added here.
- Root `Cargo.toml` — Workspace dependencies and members list. `assay-harness` path must be added to both.
- `crates/assay-core/Cargo.toml` — Reference for crate dependency structure (depends on `assay-types`).

## Constraints

- **`deny_unknown_fields` on all persisted types** — project convention, enforced on every struct that touches disk or wire.
- **Zero new workspace dependencies** — everything needed (`serde`, `schemars`, `inventory`, `chrono`, `serde_json`) is already in the workspace.
- **Zero traits** — no trait objects, no trait definitions. Closures/callbacks for control inversion (D001).
- **Leaf crate dependency direction** — `assay-harness` depends on `assay-core` + `assay-types`, never the reverse (D003, D010).
- **Type in assay-types, implementation in assay-harness** — `HarnessProfile` DTO lives in `assay-types` for cross-crate serialization; logic that operates on it lives in `assay-harness` (D010).
- **Additive MCP convention** — no existing tools are modified (D005). Not directly relevant to S02 but constrains downstream.
- **`#![deny(missing_docs)]`** — `assay-types` has this lint. Every public type and field needs doc comments.

## Common Pitfalls

- **Over-designing HarnessProfile for M002** — The type should be correct for single-agent (M001) with forward-compatible structure (e.g., `Vec<PromptLayer>` not a single string), but don't add multi-agent fields. S06's `RunManifest` with `[[sessions]]` handles the forward-compatibility for multi-agent.
- **Forgetting schema snapshot tests** — Every new type with `inventory::submit!` needs a corresponding `#[test]` in `schema_snapshots.rs` and an accepted `.snap` file. Missing this causes `cargo insta test` to fail.
- **Forgetting `pub use` re-exports** — `assay-types/src/lib.rs` re-exports all public types. New types in `harness.rs` must be re-exported or downstream crates can't use them via `assay_types::HarnessProfile`.
- **Wrong serde rename convention** — The codebase uses `kebab-case` for enums (`rename_all = "kebab-case"`) and `snake_case` is the default for struct fields. Don't mix them up.
- **Workspace member ordering** — The `members` list in root `Cargo.toml` uses `["crates/*"]` glob, so no manual member addition needed. But the `[workspace.dependencies]` section needs an `assay-harness` entry if any crate will depend on it (S03+ will).

## Open Risks

- **HarnessProfile field shape may need revision in S03** — The prompt builder and settings merger in S03 will be the first real consumers. If the type shape doesn't fit their needs, S03 will need to adjust the types. Mitigated by designing based on the boundary map's explicit contract: `prompt_layers`, `settings`, `hooks`.
- **Hook contract granularity unclear** — The roadmap mentions "pre-tool, post-tool, stop" but Claude Code's actual `hooks.json` format is unverified until S03 research. Design `HookContract`/`HookEvent` as an enum with these variants but expect S03 may add or rename variants.
- **`assay-harness` feature gate** — D002 mentions feature-gating the orchestration module. The harness crate itself doesn't need feature gating (it's a separate crate, not a module in core), but verify the `assay-core` feature gate situation if adding any orchestration integration.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust (general) | `oimiragieo/agent-studio@rust-expert` (45 installs) | available — not needed for this slice (straightforward type work) |
| Rust errors | `epicenterhq/epicenter@rust-errors` (62 installs) | available — not needed (no error types in S02) |

No skills are needed for this slice — it's workspace plumbing and type definitions following established codebase patterns.

## Sources

- Codebase exploration (patterns from `session.rs`, `feature_spec.rs`, `work_session.rs`, `schema_snapshots.rs`)
- M001 roadmap boundary map (explicit contract for what S02 produces)
- Decisions register D001, D003, D010 (architectural constraints)
