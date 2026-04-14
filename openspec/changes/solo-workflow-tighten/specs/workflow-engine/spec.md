## ADDED Requirements

### Requirement: Core workflow engine determines next action from project state
The `assay-core` crate SHALL expose a `workflow::next_action()` function that reads current project state (milestones, specs, gate history) and returns a `NextAction` enum describing what should happen next.

#### Scenario: No active milestone
- **WHEN** no milestone has status `InProgress`
- **THEN** `next_action()` returns `Idle`

#### Scenario: Spec in draft status
- **WHEN** an active milestone has a chunk whose spec status is `draft` or `ready`
- **THEN** `next_action()` returns `ReviewSpec { spec_name }` for that spec

#### Scenario: Spec approved, no gate history
- **WHEN** the active chunk's spec has status `approved` and no gate run history exists
- **THEN** `next_action()` returns `Execute { spec_name, chunk_slug }`

#### Scenario: Gates failed on last run
- **WHEN** the active chunk's most recent gate run has `required_failed > 0`
- **THEN** `next_action()` returns `FixAndRecheck { spec_name, failed_criteria }` with the names of failing criteria

#### Scenario: Gates passed, more chunks remain
- **WHEN** the active chunk's gates pass and the milestone has uncompleted chunks
- **THEN** `next_action()` returns `AdvanceChunk { milestone_slug, next_chunk }`

#### Scenario: All chunks complete
- **WHEN** all chunks in the milestone have passing gates
- **THEN** `next_action()` returns `PromptShip { milestone_slug }`

### Requirement: next_action is a pure function with no side effects
The `next_action()` function SHALL only read state from disk (milestones, specs, history). It SHALL NOT modify any files, advance any cycles, or trigger any transitions.

#### Scenario: Function does not mutate state
- **WHEN** `next_action()` is called multiple times with the same project state
- **THEN** it returns the same result each time and no files are modified

### Requirement: Strict status mode gates cycle_advance on spec status
When `[workflow] strict_status = true`, `cycle_advance` SHALL require the active chunk's spec status to be at least `approved` before running gates.

#### Scenario: Strict mode rejects draft spec
- **WHEN** `strict_status = true` and the active chunk's spec has status `draft`
- **THEN** `cycle_advance` returns an error: "Spec 'auth-flow' is still in draft status. Review and approve the spec before running gates."

#### Scenario: Strict mode allows approved spec
- **WHEN** `strict_status = true` and the active chunk's spec has status `approved` or `verified`
- **THEN** `cycle_advance` proceeds normally (evaluates gates, advances if passing)

#### Scenario: Permissive mode (default) ignores status
- **WHEN** `strict_status = false` (default) and the active chunk's spec has any status
- **THEN** `cycle_advance` runs gates regardless of spec status
