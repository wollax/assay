---
estimated_steps: 5
estimated_files: 2
---

# T02: Implement Q001–Q004 fixes

**Slice:** S01 — GitHubBackend correctness fixes (Q001–Q004)
**Milestone:** M013

## Description

Implement all four correctness fixes in `github.rs` and `factory.rs` to make the T01 contract tests pass. Q001 adds a `tracing::warn!` in the constructor for malformed repo. Q002 adds a zero-guard in `read_issue_number`. Q003 extracts the duplicated error-handling pattern into a `GhRunner::gh_error` helper. Q004 cleans milestone identifiers from the factory.rs doc comment.

## Steps

1. **Q003 — Extract `GhRunner::gh_error` helper** (do this first since it's a refactor that touches the same code as the error blocks):
   - Add `fn gh_error(&self, operation: &str, output: &std::process::Output) -> AssayError` to `GhRunner` impl block.
   - Body: `let stderr = String::from_utf8_lossy(&output.stderr);` → `tracing::warn!(repo = %self.repo, exit_code = output.status.code(), stderr = %stderr.trim(), "{operation} failed");` → `Err(AssayError::io(format!("{operation} failed: {}", stderr.trim()), "gh", std::io::Error::other(stderr.trim().to_string())))`.
   - Replace the 3 duplicated error blocks in `create_issue`, `create_comment`, `get_issue_json` with calls to `self.gh_error("gh issue create", &output)?`, etc.
   - Verify: `grep -c "from_utf8_lossy" crates/assay-backends/src/github.rs` returns `1`.

2. **Q001 — Add repo validation warn in `GitHubBackend::new`**:
   - After `Self { runner: GhRunner { repo }, label }` construction, before returning, add:
     ```rust
     if repo.is_empty() || !repo.contains('/') {
         tracing::warn!(repo = %repo, "malformed GitHub repo — expected 'owner/repo' format");
     }
     ```
   - Do NOT change the return type (D177: must remain infallible `-> Self`).
   - The warn must fire before returning Self so the subscriber captures it.

3. **Q002 — Add zero-guard in `read_issue_number`**:
   - After `let number = trimmed.parse::<u64>()...?;`, before `Ok(Some(number))`, add:
     ```rust
     if number == 0 {
         return Err(AssayError::io(
             "invalid issue number: 0 (possible file corruption)",
             &path,
             std::io::Error::new(std::io::ErrorKind::InvalidData, "issue number must be non-zero"),
         ));
     }
     ```

4. **Q004 — Clean factory.rs doc comment**:
   - Remove `(M011/S02)`, `(M011/S03)`, `(M011/S04)` from the doc comment on `backend_from_config`. Keep the surrounding text intact; just delete the milestone identifier parentheticals.

5. **Run tests**: `cargo test -p assay-backends --features github` — all 12+ tests (8 existing + 3+ new) must pass.

## Must-Haves

- [ ] `GhRunner::gh_error` helper exists and is used by all 3 methods
- [ ] Only 1 `from_utf8_lossy` call remains in `github.rs` (inside `gh_error`)
- [ ] `GitHubBackend::new` emits `tracing::warn!` for empty or slash-missing repo
- [ ] `GitHubBackend::new` remains infallible (`-> Self`, not `Result<Self>`)
- [ ] `read_issue_number` returns `Err` for `0`
- [ ] factory.rs doc has zero `(M011/S` references
- [ ] All 8 existing github_backend tests still pass

## Verification

- `cargo test -p assay-backends --features github` — all tests pass (0 failures)
- `grep -c '(M011/S' crates/assay-backends/src/factory.rs` returns `0`
- `grep -c "from_utf8_lossy" crates/assay-backends/src/github.rs` returns `1`
- Q001 traced_test assertions pass (warn log captured)
- Q002 test passes (Err returned for issue 0)

## Observability Impact

- Signals added/changed: New `tracing::warn!` event at construction time when repo is malformed (field: `repo`). Existing per-method warn events consolidated into `gh_error` helper (no behavioral change, just deduplication).
- How a future agent inspects this: `RUST_LOG=assay_backends=warn` shows malformed-repo warnings at process startup
- Failure state exposed: `read_issue_number` Err message includes path and `"0"` for diagnosability

## Inputs

- `crates/assay-backends/src/github.rs` — current implementation with 3 duplicated error blocks
- `crates/assay-backends/src/factory.rs` — doc comment with `(M011/S02)` etc.
- T01 contract tests in `crates/assay-backends/tests/github_backend.rs` — must all pass after this task

## Expected Output

- `crates/assay-backends/src/github.rs` — Q001 warn in `new()`, Q002 zero-guard in `read_issue_number`, Q003 `gh_error` helper with 3 call sites
- `crates/assay-backends/src/factory.rs` — doc comment cleaned of milestone identifiers
