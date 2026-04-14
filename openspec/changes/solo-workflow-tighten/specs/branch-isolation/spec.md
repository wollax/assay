## ADDED Requirements

### Requirement: Branch isolation is config-driven
The system SHALL support a `[workflow] auto_isolate` config setting with values `"always"`, `"never"`, or `"ask"` controlling whether work is isolated in a worktree/branch.

#### Scenario: Config defaults to ask
- **WHEN** no `[workflow]` section exists in config
- **THEN** the system defaults to `auto_isolate = "ask"`

#### Scenario: Always mode creates worktree silently
- **WHEN** `auto_isolate = "always"` and user starts work on a spec
- **THEN** the system creates a worktree without prompting

#### Scenario: Never mode skips isolation
- **WHEN** `auto_isolate = "never"` and user starts work on a spec
- **THEN** the system proceeds on the current branch without prompting

### Requirement: Ask mode uses protected branch heuristic
When `auto_isolate = "ask"`, the system SHALL detect whether the current branch is protected and prompt accordingly.

#### Scenario: On protected branch prompts for isolation
- **WHEN** the current branch is `main`, `master`, or `develop` and `auto_isolate = "ask"`
- **THEN** the system prompts: "You're on a protected branch. Create a worktree for this work?"

#### Scenario: On feature branch proceeds silently
- **WHEN** the current branch is `feature/add-auth` and `auto_isolate = "ask"`
- **THEN** the system proceeds without prompting (already isolated)

#### Scenario: Custom protected branch list
- **WHEN** config specifies `[workflow] protected_branches = ["main", "staging", "release"]`
- **THEN** only those branches trigger the isolation prompt (hardcoded defaults and dynamic detection are ignored)

### Requirement: Dynamic default branch detection supplements hardcoded list
The system SHALL call the existing `detect_default_branch()` function and add its result to the protected branch list if not already present.

#### Scenario: Non-standard default branch detected
- **WHEN** `detect_default_branch()` returns `"trunk"` and no custom `protected_branches` config exists
- **THEN** the effective protected list is `["main", "master", "develop", "trunk"]`

#### Scenario: Default branch already in hardcoded list
- **WHEN** `detect_default_branch()` returns `"main"`
- **THEN** the effective protected list remains `["main", "master", "develop"]` (no duplicate)

#### Scenario: Detection failure falls back to hardcoded only
- **WHEN** `detect_default_branch()` fails (no remote configured, etc.)
- **THEN** the system uses only the hardcoded defaults `["main", "master", "develop"]` without error
