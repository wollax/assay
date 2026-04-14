## ADDED Requirements

### Requirement: Focus skill consolidates status and next-chunk
The `/assay:focus` plugin skill SHALL replace both `/assay:status` and `/assay:next-chunk`, providing a single entry point for "what am I working on?" with spec criteria and gate status.

#### Scenario: Focus shows active spec and criteria
- **WHEN** user invokes `/assay:focus` with an active milestone
- **THEN** the skill shows the active spec name, criteria list, and gate pass/fail status

#### Scenario: Focus hides milestone wrapper for quick specs
- **WHEN** the active milestone was created via `plan quick`
- **THEN** focus output shows `Spec: <name>` without milestone or chunk terminology

#### Scenario: Focus with no active work
- **WHEN** user invokes `/assay:focus` with no active milestone
- **THEN** the skill suggests `/assay:explore` or `/assay:plan` to get started

### Requirement: Check skill replaces gate-check with smart routing
The `/assay:check` plugin skill SHALL replace `/assay:gate-check` with auto-routing gate evaluation that handles all criterion types.

#### Scenario: Check evaluates active chunk by default
- **WHEN** user invokes `/assay:check` with no arguments
- **THEN** the skill evaluates the active chunk's spec using smart gate routing

#### Scenario: Check evaluates named spec
- **WHEN** user invokes `/assay:check auth-flow`
- **THEN** the skill evaluates the named spec using smart gate routing

#### Scenario: Check reports and suggests next action
- **WHEN** gate check completes
- **THEN** the skill calls `workflow::next_action()` and communicates the result (e.g., "All criteria passed. Advance to next chunk?" or "2 criteria failed — here's what to fix")

### Requirement: Ship skill wraps gate-gated PR creation
The `/assay:ship` plugin skill SHALL create a PR with gate evidence after verifying all required gates pass.

#### Scenario: Ship with passing gates
- **WHEN** user invokes `/assay:ship` and all gates pass
- **THEN** the skill creates a PR with gate evidence in the body and a detailed check run comment

#### Scenario: Ship with failing gates
- **WHEN** user invokes `/assay:ship` and gates have failures
- **THEN** the skill reports failures and does not create a PR

### Requirement: Deprecated skills show migration notice
The old skill names (`/assay:status`, `/assay:next-chunk`, `/assay:gate-check`) SHALL remain as aliases for one version cycle, showing a deprecation notice directing to the new skill name.

#### Scenario: Old skill name shows deprecation
- **WHEN** user invokes `/assay:status`
- **THEN** the skill executes `/assay:focus` behavior but prefixes output with "Note: /assay:status is deprecated. Use /assay:focus instead."

#### Scenario: Old skill name works identically
- **WHEN** user invokes `/assay:gate-check auth-flow`
- **THEN** the skill executes `/assay:check auth-flow` behavior with a deprecation notice
