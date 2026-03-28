# S02: TrackerSource Trait, Config, & Template Manifest

**Goal:** Define the `TrackerSource` trait, `TrackerConfig` in `ServerConfig`, template manifest loading + validation + issue injection, `MockTrackerSource`, and `JobSource::Tracker` — all proven by unit tests.
**Demo:** `ServerConfig` accepts a `[tracker]` section; `TrackerSource` trait is fully defined; template manifest loads, validates, and injects issue sessions; `MockTrackerSource` exercises the full trait contract. Unit tests prove every contract.

## Must-Haves

- `TrackerSource` trait in `smelt-cli` with `poll_ready_issues()`, `transition_state()` methods using RPITIT (D019)
- `TrackerConfig` struct in `serve/config.rs` with `deny_unknown_fields`, `provider`, `manifest_template`, `poll_interval_secs`, `label_prefix`, `default_harness`, `default_timeout`
- `TrackerIssue` struct with `id`, `title`, `body`, `source_url`
- `TrackerState` enum for label-based lifecycle: `Ready`, `Queued`, `Running`, `PrCreated`, `Done`, `Failed`
- `SmeltError::Tracker { operation, message }` variant following `Forge` pattern
- `StateBackendConfig` mirror enum in `smelt-core` (D154, no Assay crate dep)
- `issue_to_manifest()` free function: clones template, injects issue as `[[session]]` entry
- Template manifest loading + validation at `ServerConfig::load()` time (D017)
- Template must have zero `[[session]]` entries (validated at startup)
- `MockTrackerSource` following `MockSshClient` VecDeque pattern
- `JobSource::Tracker` variant added to `types.rs`
- `ServerConfig::validate()` collects tracker-specific errors (D018)
- All existing 298+ tests pass (zero regressions)

## Proof Level

- This slice proves: contract
- Real runtime required: no
- Human/UAT required: no

## Verification

- `cargo test -p smelt-cli -- tracker` — all new tracker unit tests pass
- `cargo test -p smelt-core` — all core tests pass including `StateBackendConfig` serde tests
- `cargo test --workspace` — 298+ tests pass, zero regressions
- `cargo clippy --workspace -- -D warnings` — zero warnings
- `cargo doc --workspace --no-deps` — zero warnings

## Observability / Diagnostics

- Runtime signals: `SmeltError::Tracker { operation, message }` structured error variant; `tracing::info!` for template manifest load; `tracing::warn!` for template validation failures
- Inspection surfaces: `ServerConfig::load()` fails fast with collected errors on bad `[tracker]` config; `issue_to_manifest()` returns `Result` with descriptive errors
- Failure visibility: All validation errors collected (D018) and reported at startup; `SmeltError::Tracker` carries operation+message for programmatic inspection
- Redaction constraints: `token_env` fields store env var names, never values (D014/D112)

## Integration Closure

- Upstream surfaces consumed: `SmeltError` enum (smelt-core `error.rs`), `JobManifest` (smelt-core `manifest/mod.rs`), `ServerConfig` (smelt-cli `serve/config.rs`), `JobSource` enum (smelt-cli `serve/types.rs`), `MockSshClient` pattern (smelt-cli `serve/ssh/mock.rs`)
- New wiring introduced in this slice: `TrackerSource` trait + `MockTrackerSource` + `TrackerConfig` on `ServerConfig` + `issue_to_manifest()` + `StateBackendConfig` type — all contract-level, no runtime hookup
- What remains before the milestone is truly usable end-to-end: S03 (GitHub backend), S04 (Linear backend), S05 (dispatch integration, state backend passthrough, TUI, assembly)

## Tasks

- [x] **T01: Core types — TrackerIssue, TrackerState, SmeltError::Tracker, StateBackendConfig** `est:25m`
  - Why: Foundation types needed by all subsequent tasks; `StateBackendConfig` must exist before template manifest can reference it; `SmeltError::Tracker` establishes the error pattern
  - Files: `crates/smelt-core/src/error.rs`, `crates/smelt-core/src/tracker.rs`, `crates/smelt-core/src/lib.rs`, `crates/smelt-core/src/manifest/mod.rs`
  - Do: Add `SmeltError::Tracker { operation, message }` + convenience constructor following `Forge` pattern. Create `tracker.rs` in smelt-core with `TrackerIssue` struct, `TrackerState` enum (with serde + label_name() method), `StateBackendConfig` mirror enum (D154, explicit `#[serde(rename = "github")]` for GitHub variant). Add `state_backend: Option<StateBackendConfig>` to `JobManifest`. Export from lib.rs. Write unit tests for `TrackerState` label round-trip and `StateBackendConfig` serde round-trip.
  - Verify: `cargo test -p smelt-core` passes including new tests; `cargo clippy --workspace -- -D warnings` clean
  - Done when: `TrackerIssue`, `TrackerState`, `StateBackendConfig`, `SmeltError::Tracker` all compile and test; `JobManifest` accepts optional `[state_backend]`

- [x] **T02: TrackerConfig, ServerConfig integration, and TrackerSource trait** `est:30m`
  - Why: Config must parse and validate `[tracker]` section at startup; trait defines the contract S03/S04 implement
  - Files: `crates/smelt-cli/src/serve/config.rs`, `crates/smelt-cli/src/serve/tracker.rs`, `crates/smelt-cli/src/serve/mod.rs`, `crates/smelt-cli/src/serve/types.rs`
  - Do: Add `TrackerConfig` struct to `config.rs` with `deny_unknown_fields`, fields: `provider` (String), `manifest_template` (PathBuf), `poll_interval_secs` (u64, default 30), `label_prefix` (String, default "smelt"), `default_harness` (String), `default_timeout` (u64). Add `tracker: Option<TrackerConfig>` to `ServerConfig`. Add validation: provider must be "github" or "linear"; manifest_template must exist and be a file; poll_interval_secs > 0; default_timeout > 0. Create `serve/tracker.rs` with `TrackerSource` trait (RPITIT, D019): `poll_ready_issues(&self) -> impl Future<Output = Result<Vec<TrackerIssue>>> + Send`, `transition_state(&self, issue_id: &str, from: TrackerState, to: TrackerState) -> impl Future<Output = Result<()>> + Send`. Add `JobSource::Tracker` to types.rs. Register module in serve/mod.rs.
  - Verify: `cargo test -p smelt-cli -- tracker` passes; `cargo test --workspace` green; config with `[tracker]` parses; config without `[tracker]` still works
  - Done when: `ServerConfig` with `[tracker]` parses and validates; `TrackerSource` trait compiles; `JobSource::Tracker` exists

- [x] **T03: Template manifest loading, issue injection, and MockTrackerSource** `est:30m`
  - Why: Template loading + issue_to_manifest() is the core dispatch logic; MockTrackerSource enables S03/S04/S05 testing
  - Files: `crates/smelt-cli/src/serve/tracker.rs`, `crates/smelt-cli/src/serve/config.rs`
  - Do: Add `load_template_manifest(path: &Path) -> Result<JobManifest>` to tracker.rs: loads via `JobManifest::load()`, then validates: must have zero `[[session]]` entries (error if any exist). Wire into `ServerConfig::load()` so template is validated at startup. Add `issue_to_manifest(template: &JobManifest, issue: &TrackerIssue, config: &TrackerConfig) -> Result<JobManifest>`: clone template, create a `SessionDef` (name=issue.title sanitized, spec=issue.body, harness=config.default_harness, timeout=config.default_timeout), push into session vec, return. Add `MockTrackerSource` with `VecDeque<Result<Vec<TrackerIssue>>>` for poll results and `VecDeque<Result<()>>` for transitions, following MockSshClient pattern. Write tests: template with zero sessions loads OK; template with sessions rejected; issue injection produces correct manifest; mock exercises full poll→transition cycle.
  - Verify: `cargo test -p smelt-cli -- tracker` all pass; `cargo test --workspace` 298+ pass; `cargo clippy --workspace -- -D warnings` clean; `cargo doc --workspace --no-deps` clean
  - Done when: Template loads/validates, issue injection produces valid manifests, mock exercises full trait contract, all workspace tests pass

## Files Likely Touched

- `crates/smelt-core/src/error.rs`
- `crates/smelt-core/src/tracker.rs` (new)
- `crates/smelt-core/src/lib.rs`
- `crates/smelt-core/src/manifest/mod.rs`
- `crates/smelt-cli/src/serve/config.rs`
- `crates/smelt-cli/src/serve/tracker.rs` (new)
- `crates/smelt-cli/src/serve/mod.rs`
- `crates/smelt-cli/src/serve/types.rs`
