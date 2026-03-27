# S01: assay-backends crate scaffold and StateBackendConfig variants — Research

**Date:** 2026-03-27

## Summary

S01 scaffolds the `assay-backends` crate, extends `StateBackendConfig` with three named variants (`Linear`, `GitHub`, `Ssh`), regenerates schema snapshots, and introduces a `backend_from_config()` factory function that dispatches all four variants (stub backends for the three new ones, real `LocalFsBackend` for `LocalFs`).

The work is mechanically straightforward but has high blast radius: adding enum variants to `StateBackendConfig` changes two schema snapshots (D159 split), and the new crate must be wired into the workspace without disturbing the dep graph. The factory function is consumed by S02–S04 but in S01 only dispatches `LocalFs` to `LocalFsBackend` and the rest to a `NoopBackend` stub.

Requirements coverage: This slice is the primary owner of **R079** (assay-backends crate and backend factory function) and a supporting slice for R076, R077, R078 (providing the config variants they need).

## Recommendation

1. Create `crates/assay-backends/` with a `Cargo.toml` declaring `linear`, `github`, `ssh` feature flags (no deps behind them yet — S02–S04 add `reqwest` etc.).
2. Add `Linear`, `GitHub`, `Ssh` variants to `StateBackendConfig` in `assay-types/src/state_backend.rs`. Use `#[serde(rename_all = "snake_case")]` (already active on the enum).
3. Run `cargo insta review` to accept updated snapshots for both `state-backend-config-schema` and `run-manifest-orchestrate-schema`.
4. Implement `backend_from_config()` in `assay_backends::factory` — `LocalFs` → `LocalFsBackend`, others → `NoopBackend` (already exists in `assay-core`). S02–S04 replace stubs with real backends.
5. Add serde round-trip tests for all four named variants plus the `Custom` variant.
6. Do NOT wire CLI/MCP construction sites to use the factory fn in S01 — that's S04 scope (per the boundary map).

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Stub backend for unimplemented variants | `NoopBackend` in `assay_core::state_backend` | Already passes all contract tests, capabilities = none, all methods return Ok |
| Atomic file writes | `NamedTempFile` + `persist()` pattern | Established across the codebase (D062) |
| Schema snapshot testing | `insta::assert_json_snapshot!` + `schemars::schema_for!` | 80+ existing snapshot tests follow this exact pattern |
| Object safety compile guard | `_assert_object_safe(Arc<dyn StateBackend>)` | Already in `assay-core/src/state_backend.rs` |

## Existing Code and Patterns

- `crates/assay-types/src/state_backend.rs` — `StateBackendConfig` enum. Add `Linear`, `GitHub`, `Ssh` variants here. Uses `#[serde(rename_all = "snake_case")]` for tag keys, `inventory::submit!` for schema registry, `JsonSchema` derive.
- `crates/assay-core/src/state_backend.rs` — `StateBackend` trait, `CapabilitySet`, `LocalFsBackend`, `NoopBackend`. The factory fn will reference `LocalFsBackend` and `NoopBackend` from here.
- `crates/assay-types/tests/schema_snapshots.rs` — Contains `state_backend_config_schema_snapshot` (unconditional) and `run_manifest_orchestrate_schema_snapshot` (behind `#[cfg(feature = "orchestrate")]`). Both will need snapshot updates after adding variants.
- `crates/assay-core/tests/state_backend.rs` — 18 contract tests. Has `backward_compat_manifest_with_state_backend_round_trips` test showing the TOML round-trip pattern to follow.
- `crates/assay-core/src/orchestrate/executor.rs` — `OrchestratorConfig` with `backend: Arc<dyn StateBackend>`. Default impl uses placeholder `LocalFsBackend::new(PathBuf::from(".assay"))` (D157). Factory fn does NOT replace this default — it's only used at CLI/MCP dispatch sites (S04).
- `crates/assay-cli/src/commands/run.rs` — 3 `LocalFsBackend::new()` construction sites (dag, mesh, gossip). S04 wires factory fn here.
- `crates/assay-mcp/src/server.rs` — 3 `LocalFsBackend::new()` construction sites. S04 wires factory fn here.
- `plugins/` directory — Model for new crate conventions (but `assay-backends` is a workspace crate under `crates/`, not a plugin).

## Constraints

- **D001 (zero-trait convention)** — `StateBackend` is the sole exception (D149). No new traits introduced.
- **D153 (StateBackendConfig named variants)** — Variants must be named, schema-snapshot-locked. `Custom` variant preserved as escape hatch.
- **D159 (split schema snapshots)** — Both `run-manifest-schema` (non-orchestrate) and `run-manifest-orchestrate-schema` (orchestrate) snapshots affected. The `state_backend` field on `RunManifest` is behind `#[cfg(feature = "orchestrate")]` so only the orchestrate snapshot includes it. The standalone `state-backend-config-schema` snapshot is unconditional and always includes all variants.
- **D160 (assay-backends dep graph)** — `assay-backends` depends on `assay-core` + `assay-types`, not vice versa. Consistent with D003 (leaf crate pattern). `assay-cli` and `assay-mcp` will depend on `assay-backends` when S04 wires the factory fn.
- **D165 (factory fn signature)** — `pub fn backend_from_config(config: &StateBackendConfig, assay_dir: PathBuf) -> Arc<dyn StateBackend>`. Lives in `assay_backends::factory`.
- **Workspace `members = ["crates/*"]`** — The glob pattern auto-includes any new crate under `crates/`. No root `Cargo.toml` edit needed for member registration.
- **`#[serde(rename_all = "snake_case")]`** — Already on `StateBackendConfig`. New variant `Linear` serializes as `"linear"`, `GitHub` as `"git_hub"` (snake_case of "GitHub"). **CAUTION**: serde's `snake_case` converts `GitHub` to `git_hub`, not `github`. Must use `#[serde(rename = "github")]` on the variant to get the expected key, or name the variant `Github` (less idiomatic). The roadmap says `"github"` — verify with a serde round-trip test.
- **`RunManifest` has NO `deny_unknown_fields`** — confirmed by existing backward-compat tests. Adding variants to `StateBackendConfig` is safe for deserialization.

## Common Pitfalls

- **serde snake_case for "GitHub"** — `#[serde(rename_all = "snake_case")]` converts `GitHub` → `git_hub`, not `github`. Must use explicit `#[serde(rename = "github")]` on the variant, or name it `Github`. This is a common Rust serde gotcha. Verify with round-trip test before committing snapshots.
- **Schema snapshot conflicts between feature flags** — `cargo test` (no features) runs `state-backend-config-schema` snapshot. `cargo test --features orchestrate` runs `run-manifest-orchestrate-schema` snapshot. If you run `cargo insta review` under only one feature flag state, the other snapshot stays stale. Must regenerate both: `cargo test -p assay-types` and `cargo test -p assay-types --features orchestrate`, then `cargo insta review`.
- **Forgetting to add `assay-backends` to workspace dependencies** — The `members = ["crates/*"]` glob auto-includes it as a member, but if `assay-backends` needs workspace deps (e.g., `assay-core`, `assay-types`), those must be declared in its own `Cargo.toml` with `workspace = true`.
- **Factory fn returning `NoopBackend` without feature gates** — In S01, the factory fn should NOT be behind feature flags. It always compiles. The stub `NoopBackend` path for Linear/GitHub/Ssh has no conditional compilation. S02–S04 replace stubs with real backends behind `cfg(feature = ...)`.
- **Circular dependency risk** — `assay-backends` depends on `assay-core` (for `LocalFsBackend`, `NoopBackend`). `assay-core` must NOT depend on `assay-backends`. The factory fn lives in `assay-backends`, not `assay-core`. CLI/MCP import from `assay-backends`.

## Open Risks

- **`serde(rename_all = "snake_case")` interaction with struct variants** — The existing enum uses `rename_all = "snake_case"` at the enum level. For unit variants (`LocalFs`), this produces `"local_fs"`. For struct variants (`Linear { team_id, ... }`), the tag becomes `"linear"` and the inner fields follow the enum-level rename. Need to verify that `schemars` produces the expected schema shape for externally-tagged struct variants with `rename_all`. The existing `Custom` variant (a struct variant) is the precedent — confirm its schema shape matches expectations.
- **Snapshot diff size** — Adding 3 struct variants to `StateBackendConfig` produces a significant schema diff. The `run-manifest-orchestrate-schema` snapshot includes the full `StateBackendConfig` definition nested inside `RunManifest`, so the diff is doubled. `cargo insta review` will show a large diff — this is expected and correct.
- **S02–S04 may need to adjust variant field shapes** — The roadmap specifies `Linear { team_id: String, project_id: Option<String> }`, `GitHub { repo: String, label: Option<String> }`, `Ssh { host: String, remote_assay_dir: String, user: Option<String>, port: Option<u16> }`. If S02–S04 discover these need changes, the schema snapshots must be regenerated again. S01 should lock these shapes per the roadmap and boundary map.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust/serde | N/A | Core Rust — no external skill needed |
| schemars | N/A | Already used extensively in codebase |
| Rust workspace/crate management | N/A | Established patterns in codebase |

No external technologies requiring skills discovery. This slice is pure Rust crate scaffolding and type work.

## Sources

- Codebase exploration: `crates/assay-types/src/state_backend.rs`, `crates/assay-core/src/state_backend.rs`, `crates/assay-types/tests/schema_snapshots.rs`
- Decisions register: D149, D150, D153, D156, D157, D159, D160, D165
- Boundary map: M011-ROADMAP.md S01 → S02/S03/S04 section
