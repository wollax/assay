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

### Requirement: Explore loads tiered context within a ~2K token budget
The explore skill SHALL load a structured summary of project state, not raw file contents, keeping the initial context load under approximately 2000 tokens.

#### Scenario: Context loading with active project
- **WHEN** user invokes `/assay:explore` in a project with specs and milestones
- **THEN** the skill loads: config summary (project name, key settings), milestone list (names + status), spec index (names + criteria count + last gate result), and active milestone detail (chunk order, progress)

#### Scenario: Context loading for fresh project
- **WHEN** user invokes `/assay:explore` in a project with no specs or milestones
- **THEN** the skill loads only config summary and prompts "No specs defined yet. What are you building?"

#### Scenario: Full criteria loaded on demand
- **WHEN** user asks about a specific spec's criteria during exploration (e.g., "show me the auth-flow criteria")
- **THEN** the agent loads full criteria text for that spec on demand, not preemptively for all specs

#### Scenario: Context presentation is structured summary
- **WHEN** explore context is loaded
- **THEN** it is presented as a structured summary (e.g., "Project: assay | 3 specs | 1 active milestone (InProgress, chunk 2/4)"), not raw TOML file contents
