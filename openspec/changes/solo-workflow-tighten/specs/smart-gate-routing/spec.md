## ADDED Requirements

### Requirement: Gate check auto-routes by criterion kind
The gate check entry point SHALL inspect each criterion's `kind` field and route to the appropriate evaluation path without user intervention.

#### Scenario: Command criteria use shell subprocess
- **WHEN** a spec contains criteria with `kind: Command` or `kind: FileExists`
- **THEN** those criteria are evaluated via `evaluate_all()` (Path 1: shell subprocess)

#### Scenario: AgentReport criteria use evaluator subprocess by default
- **WHEN** a spec contains criteria with `kind: AgentReport`
- **THEN** those criteria are evaluated via `gate_evaluate()` (Path 3: evaluator subprocess) by default

#### Scenario: AgentReport criteria can use manual flow via config
- **WHEN** a spec contains `AgentReport` criteria and project config sets `gate.agent_eval_mode = "manual"`
- **THEN** those criteria are evaluated via `gate_run()` + `gate_report()` + `gate_finalize()` (Path 2: manual agent flow)

#### Scenario: Pipeline-only criteria are skipped
- **WHEN** a spec contains criteria with `kind: EventCount` or `kind: NoToolErrors`
- **THEN** those criteria are skipped with a clear note (not an error) since they require pipeline context

### Requirement: Mixed-kind specs evaluate all applicable criteria in one call
The system SHALL handle specs that contain a mix of Command, FileExists, and AgentReport criteria in a single gate check invocation, combining results from multiple evaluation paths.

#### Scenario: Mixed spec evaluation
- **WHEN** a spec has 3 Command criteria and 2 AgentReport criteria
- **THEN** the system evaluates Command criteria via Path 1 and AgentReport criteria via Path 3, returning a unified result with all 5 criteria outcomes

#### Scenario: Partial failure across paths
- **WHEN** Command criteria pass but AgentReport criteria fail
- **THEN** the combined result reflects the failures and the spec is not promoted to `verified`

### Requirement: Plugin skill provides single entry point
The `/assay:check` plugin skill SHALL replace `/assay:gate-check` as the sole gate evaluation entry point, handling all routing internally.

#### Scenario: Skill invoked without arguments
- **WHEN** user invokes `/assay:check` with no arguments
- **THEN** the skill evaluates the active chunk's spec (from cycle_status), routing per criterion kind

#### Scenario: Skill invoked with spec name
- **WHEN** user invokes `/assay:check auth-flow`
- **THEN** the skill evaluates the named spec, routing per criterion kind
