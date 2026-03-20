# S04: Gate-Gated PR Workflow — UAT

**Milestone:** M005
**Written:** 2026-03-20

## UAT Type

- UAT mode: live-runtime
- Why this mode is sufficient: The programmatic path (gate check logic, idempotency, TOML mutation, Verify→Complete transition) is proven by 8 integration tests using a mock `gh` binary. UAT proves the real `gh` CLI integration against an actual GitHub repo — the gap between mock and live that cannot be covered by automated tests.

## Preconditions

- `gh` CLI installed and authenticated (`gh auth status` shows logged-in account)
- An Assay project with at least one milestone in `.assay/milestones/`
- The milestone has at least one chunk spec with gates in `.assay/specs/<chunk>/gates.toml`
- On a branch that differs from the base branch (not `main` directly)
- Gates have been run recently (`assay gate run`) so history exists

## Smoke Test

```bash
assay pr create --help
```
Expected: shows positional `<MILESTONE>` argument, `--title`, `--body` options, and usage summary.

## Test Cases

### 1. Gates failing — PR not created

Set up a milestone where at least one chunk's required gate fails (or use a chunk with a failing `cmd`).

1. Run `assay pr create <milestone-slug>`
2. **Expected:** exits 1; stderr shows `"Error: gates failed for chunks: <chunk-slug> (N required failed)"` or similar structured failure message; no GitHub PR created; milestone TOML unchanged

### 2. Gates passing — PR created successfully

Set up a milestone where all chunk gates pass. Ensure you are on a non-main branch.

1. Run `assay gate run` for all chunks to confirm gates pass
2. Run `assay pr create <milestone-slug>`
3. **Expected:** exits 0; stdout shows `"PR created: #N — https://github.com/.../pull/N"`
4. Run `cat .assay/milestones/<milestone-slug>.toml`
5. **Expected:** file contains `pr_number = N` and `pr_url = "https://github.com/.../pull/N"` lines
6. Visit the PR URL in a browser
7. **Expected:** PR is open with title `"feat: <milestone-slug>"` (or custom title if `--title` was passed), targeting the configured base branch

### 3. Idempotency — second PR create returns early

Continuing from Test Case 2 (PR already created):

1. Run `assay pr create <milestone-slug>` again
2. **Expected:** exits 1; stderr shows `"Error: PR already created: #N — <url>"`; no duplicate PR created on GitHub

### 4. Custom title

1. Run `assay pr create <milestone-slug> --title "feat(auth): complete JWT foundation"`
2. **Expected:** exits 0; PR title on GitHub is `"feat(auth): complete JWT foundation"` (not the default `"feat: <slug>"`)

### 5. gh not installed / not on PATH

1. Temporarily rename or unset `gh`: `PATH=/usr/bin assay pr create <milestone-slug>` (on a milestone with all gates passing)
2. **Expected:** exits 1; stderr shows `"Error: gh CLI not found — install from https://cli.github.com"` (not a gate failure message)

### 6. Verify → Complete transition

Set up a milestone in `Verify` status with all gates passing.

1. Run `assay pr create <milestone-slug>`
2. **Expected:** PR created; milestone TOML updated with `pr_number` and `pr_url`; `status = "Complete"` in the TOML

## Edge Cases

### Missing milestone slug

1. Run `assay pr create nonexistent-slug`
2. **Expected:** exits 1; stderr shows an error indicating the milestone was not found; no crash

### Custom body

1. Run `assay pr create <milestone-slug> --body "Fixes #42. All gates green."`
2. **Expected:** PR on GitHub has the specified body text

## Failure Signals

- `assay pr create` exits 0 but no PR appears on GitHub → `gh` silently failed; check `gh pr list` and milestone TOML
- `pr_number` appears in TOML but URL is empty/wrong → JSON parse fallback triggered; inspect gh output manually
- Duplicate PR created on second invocation → idempotency check not working; inspect milestone TOML `pr_number` field

## Requirements Proved By This UAT

- R045 (Gate-gated PR creation) — live `gh pr create` call against a real GitHub repo; PR URL and number stored in milestone TOML; gate-fail path exits before touching `gh`
- R046 (Branch-per-chunk naming) — PR base branch matches `milestone.pr_base` (default `main`); current branch is the source

## Not Proven By This UAT

- Programmatic gate check logic — proven by 8 integration tests using mock `gh` binary (`test_pr_check_all_pass`, `test_pr_check_one_fails`, `test_pr_check_missing_spec`, etc.)
- Idempotency guard (`pr_number` already set) — proven by `test_pr_create_already_created` integration test
- Verify→Complete transition mechanics — proven by `test_pr_create_verify_transitions_to_complete` integration test
- MCP `pr_create` tool correctness beyond presence — integration tests cover logic; MCP transport tested by `pr_create_tool_in_router` presence test only
- `gh` authentication errors (non-zero exit with auth error in stderr) — `gh` stderr is forwarded as the error message; specific auth error wording depends on `gh` version

## Notes for Tester

- The `--body` flag passes text directly to `gh pr create --body`; long bodies should be quoted
- The mock `gh` binary tests validate the core integration path; UAT confirms real network behavior only
- If gates pass but `gh` returns a non-zero exit (e.g., existing open PR for the branch), the error message will include `gh`'s stderr verbatim — this is expected behavior
- `assay milestone list` or `cat .assay/milestones/<slug>.toml` are the primary diagnostic commands for verifying state after a PR create call
