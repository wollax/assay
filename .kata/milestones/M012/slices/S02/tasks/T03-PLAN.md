---
estimated_steps: 5
estimated_files: 3
---

# T03: Template manifest loading, issue injection, and MockTrackerSource

**Slice:** S02 — TrackerSource Trait, Config, & Template Manifest
**Milestone:** M012

## Description

Implement the template manifest loading path (`load_template_manifest`), the issue-to-manifest injection function (`issue_to_manifest`), and the `MockTrackerSource` test double. Wire template loading into `ServerConfig::load()` so bad templates fail at startup. This task completes the S02 contract — all types, traits, config, and test infrastructure are proven.

## Steps

1. Add `load_template_manifest(path: &Path) -> anyhow::Result<JobManifest>` to `serve/tracker.rs`: call `JobManifest::load()` then `validate()`, then check `manifest.session.is_empty()` — if not empty, return error "template manifest must not contain [[session]] entries". Log `tracing::info!` on successful load.
2. Add `issue_to_manifest(template: &JobManifest, issue: &TrackerIssue, config: &TrackerConfig) -> anyhow::Result<JobManifest>`: clone `template`, create `SessionDef { name: sanitize(issue.title), spec: issue.body, harness: config.default_harness.clone(), timeout: config.default_timeout, depends_on: vec![] }`, push into `manifest.session`, return. The `sanitize` helper replaces non-alphanumeric chars with hyphens and lowercases.
3. Wire template loading into `ServerConfig::load()`: after parsing and validating, if `config.tracker` is `Some`, call `load_template_manifest(&tracker.manifest_template)` and bail on failure. This satisfies D017 (validate at startup, not at dispatch time).
4. Add `MockTrackerSource` struct to `serve/tracker.rs` (behind `#[cfg(test)]`): `poll_results: Arc<Mutex<VecDeque<anyhow::Result<Vec<TrackerIssue>>>>>`, `transition_results: Arc<Mutex<VecDeque<anyhow::Result<()>>>>`. Implement `TrackerSource` for it. Add builder methods `with_poll_result()`, `with_transition_result()` following `MockSshClient` pattern.
5. Write unit tests: (a) `test_load_template_manifest_valid` — write a TOML tempfile with zero sessions, load succeeds; (b) `test_load_template_manifest_rejects_sessions` — template with `[[session]]` entries rejected; (c) `test_issue_to_manifest_injects_session` — verify injected session name, spec, harness, timeout; (d) `test_issue_to_manifest_sanitizes_title` — special chars replaced; (e) `test_mock_tracker_poll_and_transition` — mock returns configured results, exercises full cycle; (f) `test_server_config_with_tracker_section` — full round-trip parse with tracker config pointing at a valid template file.

## Must-Haves

- [ ] `load_template_manifest()` loads and validates template, rejects templates with sessions
- [ ] `issue_to_manifest()` clones template, injects session from issue with sanitized name
- [ ] Template validation happens at `ServerConfig::load()` time (D017)
- [ ] `MockTrackerSource` with VecDeque-based response queues implements `TrackerSource`
- [ ] Unit tests prove: valid template loads; sessions-present template rejected; injection correct; mock exercises full trait contract
- [ ] `cargo test --workspace` passes 298+ tests, zero regressions

## Verification

- `cargo test -p smelt-cli -- tracker` — all tracker tests pass (config + template + mock)
- `cargo test --workspace` — 298+ pass, zero failures
- `cargo clippy --workspace -- -D warnings` — zero warnings
- `cargo doc --workspace --no-deps` — zero warnings

## Observability Impact

- Signals added/changed: `tracing::info!` on successful template manifest load at startup; `SmeltError` / `anyhow` errors on template validation failure
- How a future agent inspects this: Failed template load surfaces as a startup error in `ServerConfig::load()` with descriptive message
- Failure state exposed: Template with `[[session]]` entries reports "template manifest must not contain [[session]] entries"

## Inputs

- `crates/smelt-cli/src/serve/tracker.rs` — T02's `TrackerSource` trait
- `crates/smelt-cli/src/serve/config.rs` — T02's `TrackerConfig` and `ServerConfig`
- `crates/smelt-core/src/tracker.rs` — T01's `TrackerIssue`, `TrackerState`
- `crates/smelt-core/src/manifest/mod.rs` — `JobManifest::load()`, `validate()`, `SessionDef`

## Expected Output

- `crates/smelt-cli/src/serve/tracker.rs` — gains `load_template_manifest()`, `issue_to_manifest()`, `MockTrackerSource`, and comprehensive test module
- `crates/smelt-cli/src/serve/config.rs` — `ServerConfig::load()` validates template manifest at startup
