# S01: Advanced PR creation with labels, reviewers, and templates — UAT

**Milestone:** M008
**Written:** 2026-03-23

## UAT Type

- UAT mode: live-runtime
- Why this mode is sufficient: The integration tests use mock `gh` binaries — UAT needs real `gh` against a real GitHub repo to prove labels and reviewers are actually applied.

## Preconditions

- A GitHub repository with `gh` CLI authenticated
- At least one milestone TOML in `.assay/milestones/` with specs that have passing gates
- The milestone branch exists and has diverged from main (so there's something to PR)

## Smoke Test

1. Add `pr_labels = ["test-label"]` and `pr_reviewers = ["your-github-username"]` to a milestone TOML
2. Run `assay pr create <milestone-slug>`
3. Check the created PR on GitHub — it should have the label "test-label" and "your-github-username" as a reviewer

## Test Cases

### 1. Labels from TOML

1. Edit a milestone TOML: add `pr_labels = ["ready-for-review", "automated"]`
2. Run `assay pr create <milestone-slug>`
3. **Expected:** PR on GitHub has both labels "ready-for-review" and "automated" applied

### 2. Reviewers from TOML

1. Edit a milestone TOML: add `pr_reviewers = ["teammate-username"]`
2. Run `assay pr create <milestone-slug>`
3. **Expected:** PR on GitHub has "teammate-username" requested as a reviewer

### 3. CLI labels extend TOML labels

1. Set `pr_labels = ["from-toml"]` in milestone TOML
2. Run `assay pr create <milestone-slug> --label extra-label`
3. **Expected:** PR has both "from-toml" and "extra-label" applied

### 4. Body template rendering

1. Set `pr_body_template = "# {milestone_name}\n\nSlug: {milestone_slug}\n\n## Chunks\n{chunk_list}\n\n## Gates\n{gate_summary}"` in milestone TOML
2. Run `assay pr create <milestone-slug>`
3. **Expected:** PR body on GitHub contains the milestone name, slug, bullet list of chunks, and gate pass/fail summary

### 5. Backward compatibility

1. Use an existing milestone TOML without `pr_labels`, `pr_reviewers`, or `pr_body_template` fields
2. Run `assay pr create <milestone-slug>`
3. **Expected:** PR is created normally with no labels or reviewers (same behavior as before S01)

## Edge Cases

### Invalid reviewer username

1. Set `pr_reviewers = ["nonexistent-user-12345"]` in milestone TOML
2. Run `assay pr create <milestone-slug>`
3. **Expected:** Error message from `gh` is surfaced — not a silent failure

### Empty labels list

1. Set `pr_labels = []` in milestone TOML
2. Run `assay pr create <milestone-slug>`
3. **Expected:** PR created normally with no labels (empty list treated as no labels)

## Failure Signals

- PR created but labels missing → `--label` flag not passed to `gh`
- PR created but reviewer not assigned → `--reviewer` flag not passed to `gh`
- PR body shows literal `{milestone_name}` → template rendering not invoked
- Error loading milestone TOML → backward compatibility broken by new fields

## Requirements Proved By This UAT

- R058 (Advanced PR workflow) — proves labels, reviewers, and body templates work with real `gh` and real GitHub. Partial proof (TUI PR status panel is S02).

## Not Proven By This UAT

- R058 TUI PR status panel (S02 scope)
- MCP tool invocation with labels/reviewers (would need an MCP client test, not manual UAT)
- Template rendering with all 4 placeholders simultaneously (test cases cover them individually via the full template, but edge cases in placeholder content are not exercised)

## Notes for Tester

- You'll need to clean up test PRs after running these tests (close them on GitHub)
- Labels must exist on the repo before `gh` can apply them, or `gh` will create them automatically (depending on repo settings)
- The `--reviewer` flag requires the reviewer to have access to the repo
