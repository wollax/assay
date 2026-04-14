## ADDED Requirements

### Requirement: Plan quick creates a flat spec with transparent milestone
The `assay plan quick` command (and `/assay:plan quick` skill) SHALL create a single spec with a transparent 1-chunk milestone, giving the spec full cycle/gate mechanics without exposing milestone or chunk concepts to the user.

#### Scenario: Quick plan creation
- **WHEN** user runs `assay plan quick` and provides a name and criteria
- **THEN** the system creates a milestone (slug = spec slug), a single chunk (slug = spec slug), and a spec with the provided criteria

#### Scenario: Transparent milestone is hidden in skill output
- **WHEN** the `/assay:focus` skill displays state for a quick-planned spec
- **THEN** the output shows `Spec: add-dark-mode (5 criteria)` not `Milestone: add-dark-mode, Chunk 1 of 1`

### Requirement: Quick milestones are marked as transparent
The system SHALL mark milestones created by `plan quick` with a `quick: true` flag to distinguish them from explicitly chunked milestones.

#### Scenario: Quick flag on milestone
- **WHEN** `plan quick` creates a milestone
- **THEN** the milestone TOML includes `quick = true`

#### Scenario: Milestone list annotates quick milestones
- **WHEN** `assay milestone list` includes quick milestones
- **THEN** they are annotated or filterable (e.g., `add-dark-mode (quick)`)

### Requirement: Quick-planned specs support full cycle mechanics
A spec created via `plan quick` SHALL work with `cycle_status`, `cycle_advance`, `gate_run`, and all other cycle operations identically to a chunked milestone spec.

#### Scenario: Cycle status for quick spec
- **WHEN** `cycle_status` is called with an active quick milestone
- **THEN** it returns valid status with `active_chunk_slug` matching the spec slug

#### Scenario: Cycle advance for quick spec
- **WHEN** `cycle_advance` is called and the single chunk's gates pass
- **THEN** the milestone transitions to `Verify` phase (all chunks complete)
