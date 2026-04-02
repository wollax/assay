# S04: Linear Tracker Backend — UAT

**Milestone:** M012
**Written:** 2026-03-28

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: S04 is a library slice — `LinearTrackerSource` is not yet wired into `smelt serve`'s dispatch loop (that's S05). All correctness is fully exercised by unit tests with `MockLinearClient`. Live runtime proof (real Linear project) is intentionally deferred to S05 which wires everything end-to-end.

## Preconditions

- Rust toolchain installed
- Working directory: `/Users/wollax/Git/personal/smelt`
- No external services required (all tests use MockLinearClient)
- For optional live integration: `LINEAR_API_KEY` env var set to a valid Linear personal API key; `LINEAR_TEST_TEAM_ID` set to a valid team ID

## Smoke Test

```bash
cargo test -p smelt-cli --lib -- serve::linear
```
Expected: `19 passed; 0 failed`

## Test Cases

### 1. LinearClient trait and mock tests

```bash
cargo test -p smelt-cli --lib -- serve::linear::mock
cargo test -p smelt-cli --lib -- serve::linear::compile_tests
```
1. Run commands above
2. **Expected:** 9 tests pass (8 mock + 1 compile); `MockLinearClient` returns queued results for all 5 methods; exhausted queue returns error; `LinearIssue` and `LinearLabel` deserialize from JSON correctly

### 2. LinearTrackerSource unit tests

```bash
cargo test -p smelt-cli --lib -- serve::linear::source
```
1. Run command above
2. **Expected:** 10 tests pass; poll maps LinearIssue to TrackerIssue using UUID as id; empty poll returns empty vec; transition_state performs remove then add; cache miss returns descriptive error; ensure_labels finds existing and creates missing labels

### 3. TrackerConfig Linear field validation

```bash
cargo test -p smelt-cli --lib -- serve::config
```
1. Run command above
2. **Expected:** 24 tests pass; missing `api_key_env` rejected; missing `team_id` rejected; empty/whitespace values rejected; valid linear config passes; github provider ignores linear fields; multiple errors collected in one message

### 4. Full workspace regression check

```bash
cargo test --workspace
```
1. Run command above
2. **Expected:** All tests pass, 0 failures. No regressions in smelt-core, smelt-cli lib, or integration tests.

### 5. Static analysis

```bash
cargo clippy --workspace -- -D warnings
cargo doc --workspace --no-deps
```
1. Run both commands
2. **Expected:** Both exit 0 with zero warnings

## Edge Cases

### Missing api_key_env for Linear provider

```toml
[tracker]
provider = "linear"
manifest_template = "template.toml"
# api_key_env missing
team_id = "my-team"
```
1. Load this config via `ServerConfig::load()`
2. **Expected:** Startup fails with error mentioning `api_key_env must be set when provider is "linear"`

### Empty team_id for Linear provider

```toml
[tracker]
provider = "linear"
manifest_template = "template.toml"
api_key_env = "LINEAR_API_KEY"
team_id = ""
```
1. Load this config
2. **Expected:** Startup fails with error mentioning `team_id must be set when provider is "linear"`; error collected alongside any other invalid fields

### transition_state called without ensure_labels

1. Construct `LinearTrackerSource` with mock client
2. Call `transition_state()` without calling `ensure_labels()` first
3. **Expected:** Returns `SmeltError::tracker("transition", "label '...' not in cache — was ensure_labels() called?")`

### Label already exists in Linear (ensure_labels idempotent)

1. Prime `MockLinearClient` with `find_label` returning a label UUID for each state
2. Call `ensure_labels()`
3. **Expected:** No `create_label` calls; cache populated with found UUIDs; `tracing::info!` shows `action: found` for each label

## Failure Signals

- Any test failure in `serve::linear` — indicates logic regression in client, mock, or source
- Any test failure in `serve::config` — indicates regression in TrackerConfig parsing or validation
- `cargo clippy` warnings — deny(warnings) enforces clean code
- `cargo doc` warnings — deny(missing_docs) is enforced on smelt-cli
- `reqwest` appearing only under `[dev-dependencies]` in Cargo.toml — it must be under `[dependencies]`

## Requirements Proved By This UAT

- **R071** (Tracker-driven autonomous dispatch from Linear) — `LinearTrackerSource` implements the full `TrackerSource` contract: poll issues, transition state through label lifecycle, generate manifests from templates. The dispatch path is not yet wired (S05), but the component is proven correct by mock tests.
- **R072** (TrackerSource trait abstraction) — Two concrete implementations now exist (GitHub + Linear), proving the trait is tracker-agnostic. A third tracker backend requires only a new trait impl.
- **R074** (Label-based lifecycle state machine) — All 6 lifecycle labels (smelt:ready, smelt:queued, smelt:running, smelt:pr-created, smelt:done, smelt:failed) are provisioned by `ensure_labels()` and used in `transition_state()`. Proven by unit tests.

## Not Proven By This UAT

- **Live Linear API connectivity** — All tests use MockLinearClient; actual HTTP requests to `https://api.linear.app/graphql` are not exercised. Deferred to S05 env-gated integration tests.
- **End-to-end dispatch loop** — `LinearTrackerSource` is not yet wired into `TrackerPoller` or `smelt serve`'s main loop. S05 proves this.
- **Label creation on a real Linear team** — `ensure_labels()` is tested with mocks; real GraphQL mutations against the Linear API are not verified here.
- **`state_backend` passthrough** — R075 is entirely S05's scope.
- **TUI display of tracker-sourced jobs** — S05 scope.
- **Atomic label transition safety** — The two-mutation remove+add is not atomic; race conditions between concurrent pollers are not tested.

## Notes for Tester

- The `LINEAR_API_KEY` value is never logged — only the env var name appears in tracing output. The auth header is marked `.set_sensitive(true)`.
- `ensure_labels()` must be called before `transition_state()`. If you see "not in cache" errors, that's the intended failure signal — not a bug.
- The `base_url` field in `ReqwestLinearClient` defaults to `https://api.linear.app` but is configurable for testing against a local mock server.
- `LinearIssue.description` uses `#[serde(default)]` — a null description from the API is treated as an empty string, which becomes `TrackerIssue.body`. This is intentional.
