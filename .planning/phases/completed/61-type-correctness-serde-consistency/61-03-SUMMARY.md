---
plan: 61-03
phase: 61-type-correctness-serde-consistency
type: summary
completed: "2026-04-09T16:30:00Z"
status: complete
commits:
  - hash: "21f3ceb"
    message: "feat(61-03): align CriterionKind to internally tagged serde format"
  - hash: "2180fbb"
    message: "feat(61-03): thread cli_timeout/config_timeout through evaluate_checkpoint"
  - hash: "a8d3cd2"
    message: "fix(61-03): migrate inline TOML fixtures to new CriterionKind format"
---

# 61-03 Plan Summary: CriterionKind Serde + Timeout Threading

Aligned `CriterionKind` serde tagging to internally tagged format and threaded CLI/config timeout overrides through `evaluate_checkpoint`.

## Task Results

| Task | Status | Notes |
|------|--------|-------|
| Task 1: Align CriterionKind serde tagging and migrate spec files | Complete | Added `#[serde(tag = "type", rename_all = "snake_case")]` + aliases; migrated all TOML files |
| Task 2: Thread CLI/config timeout overrides through evaluate_checkpoint | Complete | Updated `evaluate_checkpoint` and `drive_checkpoints` signatures; all call sites updated |

## Commits

- `21f3ceb` — feat(61-03): align CriterionKind to internally tagged serde format
- `2180fbb` — feat(61-03): thread cli_timeout/config_timeout through evaluate_checkpoint
- `a8d3cd2` — fix(61-03): migrate inline TOML fixtures to new CriterionKind format

## Deviations

- **Extra commit needed**: Inline TOML test fixtures in `assay-cli`, `assay-mcp/tests/mcp_handlers.rs`, and `assay-mcp/src/server.rs` also used the old bare-string `kind = "AgentReport"` format. These were discovered during `just ready` and fixed in a separate commit. The search in the plan was too narrow (only covered `.assay/` and `examples/`).

- **Clippy `too_many_arguments`**: Adding `cli_timeout` + `config_timeout` to `drive_checkpoints` pushed argument count to 8, exceeding the clippy limit of 7. Added `#[allow(clippy::too_many_arguments)]` rather than introducing a new struct, as the params are consistent with the existing function signature pattern.

## Files Modified

- `crates/assay-types/src/criterion.rs` — `CriterionKind` internally tagged serde, PascalCase aliases, new tests
- `crates/assay-types/src/gates_spec.rs` — Updated test assertion for new serialization format
- `crates/assay-types/tests/snapshots/schema_snapshots__criterion-kind-schema.snap` — Updated schema snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__criterion-schema.snap` — Updated schema snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__gate-criterion-schema.snap` — Updated schema snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__gates-spec-schema.snap` — Updated schema snapshot
- `.assay/specs/self-check.toml` — Migrated `kind = "AgentReport"` → `kind = { type = "agent_report" }`
- `examples/close-the-loop/gates.toml` — Migrated `NoToolErrors` and `EventCount` to new format
- `crates/assay-core/src/gate/mod.rs` — `evaluate_checkpoint` with `cli_timeout`/`config_timeout` params
- `crates/assay-core/src/pipeline_checkpoint.rs` — `drive_checkpoints` with timeout threading
- `crates/assay-core/src/pipeline.rs` — Updated `drive_checkpoints` call site
- `crates/assay-cli/src/commands/spec.rs` — Fixture migration
- `crates/assay-mcp/tests/mcp_handlers.rs` — Fixture migration (6 occurrences)
- `crates/assay-mcp/src/server.rs` — Fixture migration (3 occurrences)

## Verification

- `just fmt-check` — pass
- `just lint` — pass  
- `rtk cargo nextest run --workspace --exclude smelt-cli` — 2003 passed, 2 skipped
- `just deny` — pass
- Docker-dependent `smelt-cli::docker_lifecycle` tests fail without Docker daemon — pre-existing, not related to this work
