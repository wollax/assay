## ADDED Requirements

### Requirement: Gate evidence renders per surface
The system SHALL provide rendering functions that format `GateRunRecord` data for each supported surface: terminal, in-agent (collapsed), PR body, and PR check run.

#### Scenario: Terminal rendering is minimal
- **WHEN** gate results are rendered for terminal output
- **THEN** output is a 1-line summary: `"✓ 5/5 criteria passed (3 command, 2 agent-report) in 12.4s"`

#### Scenario: In-agent rendering uses collapsed blocks
- **WHEN** gate results are rendered for an agent harness (Claude Code, Codex, OpenCode)
- **THEN** output uses a collapsed/summary format showing pass/fail with expandable criterion details

#### Scenario: PR body rendering includes run ID
- **WHEN** gate results are rendered for a PR body
- **THEN** output includes summary counts, run ID, and duration as a markdown block

#### Scenario: PR check run rendering is full detail
- **WHEN** gate results are rendered as a PR check run or comment
- **THEN** output includes per-criterion pass/fail, evidence excerpts, and collapsible sections for verbose output

### Requirement: Gate results as PR check run ships out of the box
The `pr_create` workflow SHALL include full gate results as a PR comment or check run by default, not just a summary in the PR body.

#### Scenario: PR creation includes gate comment
- **WHEN** `pr_create` is called after gates pass
- **THEN** a PR comment with full criterion-by-criterion results is posted alongside the PR

#### Scenario: Gate comment uses collapsible sections
- **WHEN** the gate comment includes criteria with long stdout/stderr evidence
- **THEN** evidence is wrapped in `<details>` tags for collapsibility
