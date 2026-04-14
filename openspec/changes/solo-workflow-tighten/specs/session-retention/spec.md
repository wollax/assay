## ADDED Requirements

### Requirement: WorkSession retention is configurable
The system SHALL support configurable retention limits for WorkSessions via `.assay/config.toml` with both count-based and age-based limits.

#### Scenario: Default retention when unconfigured
- **WHEN** no `[sessions]` section exists in config
- **THEN** the system uses defaults: `max_count = 100`, `max_age_days = 90`

#### Scenario: Custom retention config
- **WHEN** config specifies `[sessions] max_count = 50, max_age_days = 30`
- **THEN** the system enforces both limits, whichever is hit first

### Requirement: Session eviction runs lazily
The system SHALL evict expired sessions during `session_create` and `session_list` operations, not via background daemon.

#### Scenario: Eviction on session_create
- **WHEN** `session_create` is called and 120 sessions exist with `max_count = 100`
- **THEN** the 20 oldest sessions are deleted before the new session is created

#### Scenario: Eviction on session_list
- **WHEN** `session_list` is called and sessions older than `max_age_days` exist
- **THEN** expired sessions are deleted before the list is returned

#### Scenario: Age and count combined
- **WHEN** 80 sessions exist (under max_count) but 30 are older than max_age_days
- **THEN** the 30 aged-out sessions are deleted, leaving 50

### Requirement: Eviction preserves sessions linked to active milestones
The system SHALL NOT evict sessions that are linked to an `InProgress` milestone, regardless of age or count limits.

#### Scenario: Active milestone protects old sessions
- **WHEN** a session is 120 days old but linked to an `InProgress` milestone
- **THEN** the session is not evicted
