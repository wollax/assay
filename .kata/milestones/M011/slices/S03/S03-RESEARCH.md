# S03: GitHubBackend — Research

**Date:** 2026-03-27

## Summary

GitHubBackend implements `StateBackend` by shelling out to the `gh` CLI for all GitHub operations — no direct REST/GraphQL API calls needed (D162). The pattern closely mirrors S02's LinearBackend: one issue per `run_dir`, first `push_session_event` creates the issue, subsequent calls append comments, `read_run_state` reads the latest comment body. The key difference is transport: subprocess calls via `std::process::Command` instead of `reqwest::blocking::Client`, which means no async runtime concerns, no new HTTP deps, and simpler error handling.

The mock `gh` binary pattern already exists in `crates/assay-core/tests/pr.rs` — the `write_fake_gh()` / `with_mock_gh_path()` helpers create shell scripts that impersonate `gh` by echoing canned output. S03 can reuse this exact approach for contract tests. The main design question is `gh issue create`'s output format: unlike `gh pr create --json`, `gh issue create` has no `--json` flag. It outputs a plain URL to stdout (e.g. `https://github.com/owner/repo/issues/42`). The issue number must be parsed from this URL.

Capabilities should be all-false per S03-CONTEXT.md. Unlike LinearBackend (which sets `annotations=true` and posts tagged comments for `annotate_run`), GitHubBackend has no meaningful annotation concept — gossip manifest paths are local filesystem paths irrelevant to remote GitHub observers. All unsupported methods (`send_message`, `poll_inbox`, `annotate_run`, `save_checkpoint_summary`) return `Ok(())` / `Ok(None)` / `Ok(vec![])` — consistent with NoopBackend's silent-success pattern for test/degradation use, but alternative: return `Err` for unsupported methods to match the trait contract. Recommend: return `Err` for clarity, matching LinearBackend's pattern for `send_message`/`poll_inbox` but allowing `annotate_run`/`save_checkpoint_summary` to silently no-op since capability flags are false and callers gate on them.

## Recommendation

Follow the LinearBackend structure closely but substitute `std::process::Command` for `reqwest::blocking::Client`:

1. **`GhRunner` struct** (analogous to `LinearClient`): wraps `std::process::Command` calls. Methods: `create_issue(repo, title, body, label) -> u64`, `create_comment(repo, issue_number, body)`, `get_issue_comments(repo, issue_number) -> Option<String>`. All methods take `&self` for the repo string, or repo is stored on the struct.

2. **Issue tracking**: cache issue number in `run_dir/.github-issue-number` (parallel to LinearBackend's `.linear-issue-id`). First `push_session_event` → `gh issue create`, subsequent → `gh issue comment`.

3. **`read_run_state`**: use `gh issue view <number> --repo <repo> --json body,comments` to get both the issue body and comments in one call. If comments exist, deserialize the last comment's body as `OrchestratorStatus` JSON. If no comments, deserialize the issue body (the first `push_session_event` wrote state as the issue body).

4. **Mock `gh` binary tests**: create a more sophisticated mock script than `write_fake_gh` — one that inspects `$1 $2` (`issue create`, `issue comment`, `issue view`) and responds appropriately. Alternatively, write multiple mock scripts and swap them per test. The `with_mock_gh_path` + `#[serial]` pattern from `pr.rs` is the proven approach.

5. **No new deps**: `std::process::Command` is stdlib. No `reqwest`, no `tokio`. The `github` feature flag in Cargo.toml stays empty (`github = []`).

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Mock `gh` binary in tests | `write_fake_gh()` + `with_mock_gh_path()` in `crates/assay-core/tests/pr.rs` | Battle-tested PATH override pattern with `#[serial]` safety; extend for multi-subcommand dispatch |
| Atomic issue-number file I/O | `LinearBackend::read_issue_id()` / `write_issue_id()` pattern | Same file-per-run-dir tracking; just change filename from `.linear-issue-id` to `.github-issue-number` |
| `StateBackend` error patterns | `AssayError::io()` / `AssayError::json()` helpers | Consistent error formatting across all backends |
| `gh` non-zero exit handling | `pr_create_if_gates_pass` in `crates/assay-core/src/pr.rs` | Same stderr-capture + error-wrapping pattern for `Command` output |

## Existing Code and Patterns

- `crates/assay-backends/src/linear.rs` — **Primary template.** 422 lines. LinearClient wraps reqwest; GitHubBackend wraps Command. Same `push_session_event` first-call/subsequent-call pattern. Same `read_run_state` → latest-comment pattern. Same `.xxx-issue-id` file caching. Follow this structure nearly line-for-line.
- `crates/assay-backends/src/factory.rs` — Factory fn needs the `GitHub` arm updated from `NoopBackend` to `GitHubBackend::new(...)`. Use `#[cfg(feature = "github")]` / `#[cfg(not(feature = "github"))]` dual-arm pattern (same as S02 did for `linear`).
- `crates/assay-core/src/pr.rs` — `write_fake_gh()`, `with_mock_gh_path()`, `parse_gh_output()` — proven patterns for mock `gh` and output parsing. The URL-to-number parser is directly reusable.
- `crates/assay-core/src/state_backend.rs` — Trait definition, `NoopBackend` reference, `LocalFsBackend` reference for method signatures and error patterns.
- `crates/assay-backends/Cargo.toml` — `github = []` feature flag already declared (no deps needed). Add `serial_test` to dev-deps for `#[serial]` on mock-gh tests.

## Constraints

- **D162**: All GitHub operations via `gh` CLI. No direct REST/GraphQL API calls. `gh issue create`, `gh issue comment`, `gh issue view --json`.
- **D077**: Use `--json` for stable machine-readable output where available. `gh issue view --json body,comments` is available. `gh issue create` does NOT have `--json` — must parse URL from stdout.
- **D007/D150**: Sync core. `std::process::Command` is already sync — no runtime bridges needed. Simpler than LinearBackend.
- **D008**: CLI-first subprocess pattern. Use `Command::arg()` chaining (not shell string interpolation). `--repo <repo>` flag ensures the command works from any working directory.
- **No reqwest dep**: The `github` feature flag should NOT pull in `reqwest`. All I/O via `gh` subprocess. This keeps the feature flag lightweight.
- **`serial_test` for mock-gh tests**: PATH modification is global state — all tests using `with_mock_gh_path` must be `#[serial]`. Add `serial_test` to `assay-backends` dev-deps.

## Common Pitfalls

- **`gh issue create` has no `--json` flag** — The URL is printed to stdout as plain text (e.g. `https://github.com/owner/repo/issues/42\n`). Must parse the issue number from the URL's last path segment. Edge case: URL may have a trailing newline or carriage return. Use `.trim()` + `.rsplit('/').next()` + `.parse::<u64>()`.
- **`gh issue view --json comments` returns the full comment array, not just the latest** — Must index into the array. `comments` field is an array of objects with `body` field. Take the last element: `comments.last()`. If empty, fall back to the issue `body` field.
- **`gh issue comment` writes to stdin if no `--body` flag** — Always pass `--body <text>`. If the body is very large (full OrchestratorStatus JSON can be 10KB+), command-line length limits may be hit. Mitigation: use `--body-file -` and pipe via stdin instead of `--body` arg. This avoids OS-level `ARG_MAX` limits.
- **Mock `gh` script must handle multiple subcommands** — A single `write_fake_gh` that always echoes the same output won't work for tests that call `create` then `comment` in sequence. Write a dispatcher script that inspects `$1 $2` (e.g. `issue create` vs `issue comment` vs `issue view`) and responds with different canned output per subcommand.
- **`--repo` flag required for cross-directory operation** — `gh issue create` without `--repo` uses the git remote of the current directory. `GitHubBackend` must always pass `--repo <repo>` explicitly since `run_dir` is under `.assay/orchestrator/`, not the project repo root.
- **`gh` not installed or not authenticated** — `gh` missing from PATH returns `ErrorKind::NotFound` from `Command::new("gh").spawn()`. Unauthenticated `gh` returns non-zero exit with stderr like "To get started with GitHub CLI, please run: gh auth login". Both are `AssayError::Io` — distinguish by error message for user actionability.

## Open Risks

- **Command-line argument length for `--body`** — OrchestratorStatus JSON can exceed 100KB for large runs with many sessions. `ARG_MAX` on macOS is 1MB, Linux is typically 2MB, but individual argument limits may be lower. Using `--body-file -` with stdin piping is safer but adds complexity (need `Stdio::piped()` + write + wait). Recommend: use stdin piping via `--body-file -` from the start to avoid discovering this limit in production.
- **`gh issue create` URL parsing fragility** — If GitHub ever changes the URL format (e.g. GitHub Enterprise with different path structure), the parser breaks. Mitigation: parse only the numeric suffix after the last `/`, validate it's a positive integer. The `--repo owner/repo` flag makes the URL format predictable for github.com.
- **Test isolation with `#[serial]`** — Mock `gh` tests modify the global PATH env var. If any test panics between setting and restoring PATH, subsequent tests in the same process may see the wrong PATH. The `with_mock_gh_path` pattern in `pr.rs` handles this but doesn't use a panic guard (no `Drop` cleanup). For robustness, consider using `serial_test` crate's `#[serial]` attribute (already a dev-dep of `assay-core`). Need to add it to `assay-backends` dev-deps too.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| GitHub CLI (gh) | N/A | No agent skill needed — well-understood subprocess pattern |
| Rust std::process::Command | N/A | Core stdlib, no skill needed |

## Sources

- `crates/assay-backends/src/linear.rs` — LinearBackend implementation (422 lines, primary template for GitHubBackend)
- `crates/assay-core/src/pr.rs` — Existing `gh` CLI patterns: `write_fake_gh()`, `with_mock_gh_path()`, `parse_gh_output()`, error handling
- `crates/assay-core/tests/pr.rs` — Mock `gh` binary test infrastructure with PATH override and `#[serial]`
- `crates/assay-backends/src/factory.rs` — Factory fn dispatch pattern with `#[cfg(feature)]` dual arms
- `.kata/milestones/M011/slices/S03/S03-CONTEXT.md` — Slice scope, constraints, capabilities decision
- `gh issue create --help` / `gh issue view --help` / `gh issue comment --help` — CLI flag reference
