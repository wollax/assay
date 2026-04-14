## ADDED Requirements

### Requirement: Specs have a lifecycle status field
The `GatesSpec` struct SHALL include an optional `status` field with defined enum values: `draft`, `ready`, `approved`, `verified`.

#### Scenario: New spec defaults to draft
- **WHEN** a spec is created via `spec_create` or `gate_wizard` without an explicit status
- **THEN** the spec's status field is `None`, which the workflow engine treats as `draft`

#### Scenario: Existing specs without status field
- **WHEN** an existing `gates.toml` without a `status` field is loaded
- **THEN** deserialization succeeds with `status: None` (treated as `draft`)

### Requirement: Spec status transitions follow defined rules
The system SHALL enforce valid status transitions: `draft → ready → approved → verified`. Backward transitions (e.g., `verified → draft`) SHALL be allowed for rework scenarios.

#### Scenario: Valid forward transition
- **WHEN** a spec with status `draft` is updated to `ready`
- **THEN** the transition succeeds and the spec is saved

#### Scenario: Skip transition
- **WHEN** a spec with status `draft` is updated directly to `approved`
- **THEN** the transition succeeds (skipping intermediate states is allowed for flexibility)

#### Scenario: Backward transition for rework
- **WHEN** a spec with status `verified` is updated to `draft`
- **THEN** the transition succeeds to support rework after requirements change

### Requirement: Passing gate run auto-promotes spec to verified
The system SHALL automatically set a spec's status to `verified` when a gate run completes with all required criteria passing.

#### Scenario: Gate pass promotes status
- **WHEN** `gate_run` completes for spec `auth-flow` with `required_failed == 0`
- **THEN** the spec's status is updated to `verified` and saved to disk

#### Scenario: Gate failure does not change status
- **WHEN** `gate_run` completes for spec `auth-flow` with `required_failed > 0`
- **THEN** the spec's status remains unchanged

#### Scenario: Advisory failures do not block promotion
- **WHEN** `gate_run` completes with `required_failed == 0` but `advisory_failed > 0`
- **THEN** the spec's status is promoted to `verified` (advisory failures are informational)
