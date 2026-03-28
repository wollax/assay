# S02: TrackerSource Trait, Config, & Template Manifest — UAT

**Milestone:** M012
**Written:** 2026-03-28

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: S02 is a contract-level slice with no runtime hookup required. All deliverables are types, traits, config parsing, and template loading logic — fully exercised by unit tests. No Docker, no external APIs, no human interaction needed at this stage. Real runtime proof is delegated to S03 (GitHub backend) and S04 (Linear backend).

## Preconditions

- Rust toolchain available (`cargo test --workspace` can run)
- Workspace compiles cleanly
- No `SMELT_K8S_TEST`, `SMELT_TEST_SSH_KEY`, or other integration-test env vars needed

## Smoke Test

```sh
cargo test -p smelt-cli --lib -- serve::tracker
```

Expected: 14 tests pass, 0 failures. If this passes, the core deliverables (TrackerSource trait, template loading, issue injection, MockTrackerSource) are all wired correctly.

## Test Cases

### 1. Config with `[tracker]` section parses and validates

```toml
# server.toml excerpt
[tracker]
provider = "github"
manifest_template = "./template.toml"
poll_interval_secs = 60
label_prefix = "smelt"
default_harness = "claude"
default_timeout = 300
```

1. Create a valid template TOML with no `[[session]]` entries
2. Load `ServerConfig` pointing to the above config
3. Call `ServerConfig::validate()`
4. **Expected:** Config parses and validates without errors; `config.tracker` is `Some(TrackerConfig { provider: "github", ... })`

### 2. Config without `[tracker]` section still parses

1. Load a `ServerConfig` from an existing `server.toml` without a `[tracker]` section
2. **Expected:** Config parses correctly; `config.tracker` is `None`; no validation errors related to tracker

### 3. Invalid tracker config reports all errors

```toml
[tracker]
provider = "jira"          # invalid
manifest_template = "/nonexistent/path.toml"
poll_interval_secs = 0     # invalid
label_prefix = ""          # invalid
default_harness = ""       # invalid
default_timeout = 0        # invalid
```

1. Load config with the above `[tracker]` section
2. Call `ServerConfig::validate()`
3. **Expected:** Returns an error containing all validation failures in a single message (provider invalid, poll_interval > 0 required, default_timeout > 0 required, default_harness non-empty required, label_prefix non-empty required) — not fail-fast

### 4. Template manifest with `[[session]]` entries rejected at startup

1. Create a template TOML containing a `[[session]]` entry
2. Set `manifest_template` to this file in `[tracker]` config
3. Call `ServerConfig::load()`
4. **Expected:** Error returned with message containing "must not contain [[session]] entries"

### 5. Template manifest without sessions loads successfully

1. Create a valid `JobManifest` TOML with no `[[session]]` entries
2. Call `load_template_manifest()` with the path
3. **Expected:** Returns `Ok(JobManifest)` with empty session vec; `tracing::info!` logged with file path

### 6. `issue_to_manifest()` injects session from TrackerIssue

1. Load a template manifest (no sessions)
2. Create a `TrackerIssue { id: "42", title: "Fix the thing!", body: "...", source_url: "..." }`
3. Call `issue_to_manifest(&template, &issue, &config)` where `config.default_harness = "claude"` and `config.default_timeout = 300`
4. **Expected:** Returns manifest with exactly 1 session; session name is sanitized title ("fix-the-thing"); session spec is issue body; harness and timeout match config values; template fields (environment, forge, etc.) are preserved

### 7. MockTrackerSource exercises poll→transition cycle

1. Create `MockTrackerSource::new()` with two poll results queued (one list of issues, one empty)
2. Call `poll_ready_issues()` twice
3. Call `transition_state("42", Ready, Queued)` with a queued success result
4. **Expected:** First poll returns the issue list; second poll returns empty vec; transition returns Ok(()); no panics; deque is exhausted

### 8. TrackerState label names are correct

1. Call `TrackerState::Ready.label_name("smelt")`
2. Call `TrackerState::Queued.label_name("smelt")`
3. Call `TrackerState::Failed.label_name("myprefix")`
4. **Expected:** `"smelt:ready"`, `"smelt:queued"`, `"myprefix:failed"`

### 9. StateBackendConfig round-trips through TOML

```toml
[state_backend]
type = "linear"
# ... linear fields
```

1. Parse a `JobManifest` TOML containing a `[state_backend]` section
2. Serialize back to TOML
3. **Expected:** Round-trip produces identical structure; `state_backend` field is `Some(StateBackendConfig::Linear { ... })`

## Edge Cases

### Template with `deny_unknown_fields` rejects unknown fields

1. Create a template TOML with an unknown top-level field
2. Call `load_template_manifest()`
3. **Expected:** TOML parse error (not a panic); error message names the unknown field

### `issue_to_manifest()` title sanitization edge cases

1. Issue title: `"  --hello world!!  "` (leading/trailing hyphens, spaces, special chars)
2. Call `issue_to_manifest()`
3. **Expected:** Session name is `"hello-world"` (trimmed, collapsed hyphens, lowercase, non-alnum replaced)

### `JobManifest` without `state_backend` parses as `None`

1. Parse any existing job manifest TOML (no `[state_backend]` section)
2. **Expected:** `manifest.state_backend` is `None`; no parse error (backward compatible via `serde(default)`)

## Failure Signals

- Any of the `serve::tracker` or `serve::config` tests failing indicates a regression in the contract
- `cargo clippy --workspace -- -D warnings` producing warnings indicates documentation or lint drift
- `cargo doc --workspace --no-deps` producing warnings indicates broken doc links
- `ServerConfig::load()` not failing on a template with `[[session]]` entries indicates the startup validation wiring is broken
- `issue_to_manifest()` producing a manifest with 0 sessions indicates the injection is broken

## Requirements Proved By This UAT

- R072 (TrackerSource trait abstraction) — Trait compiles, is implementable (DummySource + MockTrackerSource prove both), RPITIT async methods work without `#[async_trait]`
- R073 (Template manifest with issue injection) — `load_template_manifest()` + `issue_to_manifest()` unit-tested end-to-end; injection preserves template infrastructure and adds the correct session
- R074 (Label-based lifecycle state machine) — `TrackerState` enum with all 6 variants and `label_name()` method proven by unit tests; label strings match the `{prefix}:{state}` format used by both GitHub and Linear

## Not Proven By This UAT

- R070 (GitHub tracker end-to-end dispatch) — requires GithubTrackerSource (S03), real `gh` CLI, and a GitHub repo
- R071 (Linear tracker end-to-end dispatch) — requires LinearTrackerSource (S04), real Linear API, and a project
- R074 (Label lifecycle in live tracker) — labels are correctly named but actual label creation/transition in GitHub or Linear is not proven until S03/S04
- R075 (State backend passthrough into RunManifest) — `state_backend` field exists on `JobManifest` and serializes, but the passthrough into AssayInvoker's RunManifest generation is S05 work
- Operational behavior of `smelt serve --config server.toml` with a real `[tracker]` section (TrackerPoller not yet integrated into dispatch loop)
- Performance under concurrent polling or high-frequency issue creation
- Double-dispatch prevention (D157) — the `smelt:ready` → `smelt:queued` atomic transition is designed in S02 but implemented in S03

## Notes for Tester

- All proofs are via `cargo test` — no manual steps required for this UAT
- The `MockTrackerSource` is `pub(crate)` under `#[cfg(test)]` — it exists only in test builds and is not accessible from integration test files (only from `--lib` tests within `smelt-cli`)
- Template manifest loading at startup is wired into `ServerConfig::load()` (not just `validate()`) — to test it, call `load()` with a `[tracker]` section pointing to a real file
- The `sanitize()` function is private — its behavior is only observable through `issue_to_manifest()` return values
