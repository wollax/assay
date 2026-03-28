# S03: GitHub Issues Tracker Backend — Research

**Date:** 2026-03-28

## Summary

S03 implements `GithubTrackerSource: TrackerSource` — the concrete GitHub Issues backend that polls for issues via `gh issue list`, transitions lifecycle labels via `gh issue edit --add-label/--remove-label`, and auto-creates missing lifecycle labels via `gh label create`. The implementation follows the `SubprocessSshClient` pattern (subprocess shell-out with `tokio::process::Command`, `which::which` for binary discovery, and a generic `<G: TrackerSource>` trait parameter for testability with mocks).

The `TrackerSource` trait and all supporting types (`TrackerIssue`, `TrackerState`, `TrackerConfig`, `issue_to_manifest()`, `MockTrackerSource`) are already fully implemented in S02. S03's job is to wire up the `gh` CLI as the concrete backend, handle error cases (missing binary, expired auth, rate limiting, empty results), and prove correctness via unit tests with a mock `gh` wrapper plus integration tests gated by `SMELT_GH_TEST=1`.

The `TrackerConfig` currently has no GitHub-specific fields (repo name, etc.). S03 will need to either extend `TrackerConfig` with provider-specific fields or introduce a separate config struct. The simplest path: add an optional `repo` field (`Option<String>`) to `TrackerConfig` — required when `provider = "github"`, validated at startup. This follows the existing `deny_unknown_fields` + D018 error collection pattern.

## Recommendation

**Approach: `gh` CLI subprocess with a thin `GhClient` trait for testability.**

1. Define a `GhClient` trait with methods: `list_issues(repo, label, json_fields) -> Result<Vec<GhIssue>>`, `add_label(repo, issue_number, label) -> Result<()>`, `remove_label(repo, issue_number, label) -> Result<()>`, `create_label(repo, label_name) -> Result<()>`.
2. Implement `SubprocessGhClient` that shells out to `gh` (mirrors `SubprocessSshClient`).
3. Implement `GithubTrackerSource` that composes `GhClient` + `TrackerConfig` and implements `TrackerSource`.
4. Unit tests use a `MockGhClient` (VecDeque pattern from `MockSshClient`).
5. Integration tests gated by `SMELT_GH_TEST=1` and `SMELT_GH_REPO=owner/repo` env vars.

This keeps the `gh` subprocess concerns separated from the TrackerSource protocol, making both independently testable.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| JSON parsing of `gh` output | `serde_json` (already unconditional dep, D079) | `gh --json` outputs JSON; parse with serde |
| Binary discovery on PATH | `which::which` (already in workspace deps) | Same pattern as `ssh_binary()` in SubprocessSshClient |
| Async subprocess execution | `tokio::process::Command` (already in workspace) | Same pattern as SSH client |
| Session name sanitization | `sanitize()` in `serve/tracker.rs` | Already written and tested in S02 |
| Issue-to-manifest injection | `issue_to_manifest()` in `serve/tracker.rs` | Already written and tested in S02; S03 should NOT reimplement |

## Existing Code and Patterns

- `crates/smelt-cli/src/serve/ssh/client.rs` — **Primary pattern to follow.** `SubprocessSshClient` wraps `ssh`/`scp` binaries with `which::which` discovery, `tokio::process::Command` execution, structured output parsing. `GhClient` should mirror this exactly.
- `crates/smelt-cli/src/serve/ssh/mock.rs` — **Mock pattern to follow.** `MockSshClient` uses `Arc<Mutex<VecDeque<Result>>>` for each method. `MockGhClient` should use the same pattern.
- `crates/smelt-cli/src/serve/ssh/mod.rs` — **Module structure to follow.** `SshClient` trait + `SubprocessSshClient` + mock + operations. S03 should create `serve/github/` with `mod.rs` (trait), `client.rs` (subprocess impl), `mock.rs` (test double).
- `crates/smelt-cli/src/serve/tracker.rs` — **TrackerSource trait, `issue_to_manifest()`, `load_template_manifest()`** — all S02 outputs that S03 consumes directly. `MockTrackerSource` is in `tracker::mock`.
- `crates/smelt-cli/src/serve/config.rs` — **TrackerConfig** with `deny_unknown_fields`. Needs `repo: Option<String>` for GitHub. Validation collects errors per D018.
- `crates/smelt-core/src/tracker.rs` — **TrackerIssue, TrackerState** — S03 maps `gh` JSON output to `TrackerIssue` and uses `TrackerState::label_name(prefix)` for label strings.
- `crates/smelt-core/src/error.rs` — **SmeltError::Tracker** variant with `tracker()` constructor — use for structured errors.

## Constraints

- **`deny_unknown_fields` on `TrackerConfig`** — any new field (e.g. `repo`) must be added to the struct or parsing breaks for existing valid configs. Use `Option<String>` + validation-time enforcement.
- **D155: GitHub tracker uses `gh` CLI, not octocrab** — `gh` is for issue operations (list, label); PR creation still uses octocrab via ForgeClient. Don't pull octocrab into tracker code.
- **D157: Double-dispatch prevention** — first action on pickup must be `smelt:ready → smelt:queued` label transition before enqueueing. If the label swap fails, skip the issue.
- **D002: No Assay crate dependency** — all types stay Smelt-side.
- **RPITIT (D019)** — `TrackerSource` trait uses `async fn` directly (not `#[async_trait]`). `GithubTrackerSource` must impl with `async fn`.
- **`gh` requires auth** — `gh auth status` must pass. Handle missing `gh` binary (which::which fails) and auth failure (gh returns non-zero with "not logged in" message) gracefully.
- **`gh -R` flag** — all `gh issue` and `gh label` commands need `-R owner/repo` to target the correct repository. This comes from the `repo` field in TrackerConfig.

## Common Pitfalls

- **`gh` JSON output parsing fragility** — `gh issue list --json` returns a JSON array. Empty results return `[]` (not an error). Ensure the parser handles empty arrays gracefully without treating them as errors.
- **Label name collisions** — `TrackerState::label_name("smelt")` produces `"smelt:pr_created"` with an underscore. GitHub label names are case-insensitive and allow any characters, so this is fine — but ensure label creation and filtering use the exact same string.
- **Rate limiting** — `gh` handles rate limiting internally (waits and retries). No special handling needed from Smelt's side for standard polling intervals (30s default). Document this assumption.
- **Non-zero exit from `gh` doesn't always mean failure** — `gh issue list` returns exit 0 with `[]` when no issues match. But `gh issue edit` returns non-zero when the issue doesn't exist or labels are invalid. Parse stderr for error messages.
- **Label auto-creation race** — if two `smelt serve` instances try to create the same label simultaneously, one will get `already exists` from `gh label create`. Use `--force` flag to make label creation idempotent.
- **`gh` not respecting `--repo` when in a git directory** — `gh` infers the repo from the current directory's git remote if `-R` is not provided. Always pass `-R` explicitly to avoid this.
- **Subprocess timeout** — `gh` CLI doesn't have a built-in timeout. If the API is unresponsive, the subprocess could hang. Wrap with `tokio::time::timeout()` on the poll operation.

## Open Risks

- **`gh` auth token expiry during long-running `smelt serve`** — `gh` caches auth tokens; if they expire mid-run, subsequent `gh` calls fail. Mitigation: surface the error clearly via `SmeltError::Tracker` and let the user re-auth. The poller will retry on the next interval.
- **Large issue counts** — `gh issue list --label smelt:ready` with hundreds of ready issues could be slow or hit pagination. Use `--limit` to cap results (e.g. 50 per poll). Document the default.
- **GitHub Enterprise / GHE compatibility** — `gh` supports GHE via `gh auth login --hostname`. The `-R` flag works the same way. No special handling needed, but worth documenting.

## TrackerConfig Extension Design

Current `TrackerConfig` has no `repo` field. For GitHub, `gh` needs `-R owner/repo`.

**Recommended approach:** Add `repo: Option<String>` to `TrackerConfig`.

```rust
/// Repository slug in `owner/repo` format (required for GitHub provider).
#[serde(default)]
pub repo: Option<String>,
```

Validation in `ServerConfig::validate()`:
- When `provider == "github"`: `repo` must be `Some` and match `owner/repo` format.
- When `provider == "linear"`: `repo` is ignored (Linear uses different identifiers).

This keeps all tracker config in one struct with `deny_unknown_fields` — no provider-specific sub-structs needed.

## `gh` CLI Command Mapping

| Operation | Command | Output |
|-----------|---------|--------|
| List ready issues | `gh issue list -R owner/repo --label smelt:ready --json number,title,body,url --limit 50` | JSON array of issues |
| Add label | `gh issue edit -R owner/repo <number> --add-label smelt:queued` | No output on success |
| Remove label | `gh issue edit -R owner/repo <number> --remove-label smelt:ready` | No output on success |
| Create label | `gh label create -R owner/repo "smelt:ready" --force` | Label created/updated |
| Check auth | `gh auth status` | Exit 0 if authenticated |

**Note:** `--add-label` and `--remove-label` can be combined in a single `gh issue edit` call:
```
gh issue edit -R owner/repo 42 --add-label smelt:queued --remove-label smelt:ready
```
This is atomic from the user's perspective (single API call under the hood).

## File Structure Plan

```
crates/smelt-cli/src/serve/
  github/
    mod.rs          — GhClient trait, GhIssue struct, GithubTrackerSource
    client.rs       — SubprocessGhClient (gh CLI wrapper)
    mock.rs         — MockGhClient for tests
  config.rs         — +repo field, +validation for github provider
  mod.rs            — +pub mod github
```

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| gh CLI | N/A | none found — `gh` is a thin CLI tool; no agent skill needed |
| Rust async subprocess | N/A | Existing `SubprocessSshClient` pattern is sufficient |

## Sources

- `gh issue list --help`, `gh issue edit --help`, `gh label create --help` — CLI help output for command syntax and flags
- S02 Summary — TrackerSource trait contract, issue_to_manifest(), MockTrackerSource patterns
- `crates/smelt-cli/src/serve/ssh/` — SubprocessSshClient pattern: binary discovery, Command execution, mock pattern
- D155 — GitHub tracker uses `gh` CLI decision
- D157 — Double-dispatch prevention via atomic label transition
