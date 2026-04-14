## ADDED Requirements

### Requirement: Explore skill loads Assay project context
The `/assay:explore` plugin skill SHALL load the current project's specs, config, milestone state, and codebase structure before entering conversational mode.

#### Scenario: Explore with existing specs
- **WHEN** user invokes `/assay:explore` in a project with specs defined
- **THEN** the skill loads spec names, criteria counts, and gate history summaries as context for the conversation

#### Scenario: Explore with no specs
- **WHEN** user invokes `/assay:explore` in an initialized project with no specs
- **THEN** the skill proceeds with project config and codebase context only, without error

#### Scenario: Explore in uninitialized project
- **WHEN** user invokes `/assay:explore` in a directory without `.assay/`
- **THEN** the skill informs the user to run `assay init` first

### Requirement: Explore is conversational with no fixed structure
The explore skill SHALL operate as a thinking partner with no mandatory steps, no required outputs, and no enforced sequence. The skill prompt defines a stance, not a workflow.

#### Scenario: Free-form exploration
- **WHEN** user provides a vague idea ("real-time collaboration") as input
- **THEN** the agent asks clarifying questions, surfaces tradeoffs, and follows the user's lead without forcing a specific path

#### Scenario: Transition to planning
- **WHEN** the user indicates requirements have crystallized (e.g., "ready to plan", "let's build this")
- **THEN** the agent offers to invoke `/assay:plan` with the gathered context
