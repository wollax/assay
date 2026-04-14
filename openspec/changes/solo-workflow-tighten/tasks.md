## 1. Spec Status Field (assay-types + assay-core)

- [x] 1.1 Add `SpecStatus` enum (`Draft`, `Ready`, `Approved`, `Verified`) to `assay-types`
- [x] 1.2 Add optional `status` field to `GatesSpec` with `#[serde(default)]` — existing specs deserialize as `None` (treated as `Draft`). **Decision Q1: safe default, no inference from history.**
- [x] 1.3 Add `spec_set_status()` function in `assay-core/spec` for validated transitions
- [x] 1.4 Add auto-promotion logic in gate evaluation: on all-required-pass, set spec status to `Verified`
- [x] 1.5 Add `status` filter to `spec_list` MCP tool (e.g., `spec_list(status: "draft")`)
- [x] 1.6 Add optional `uat: Option<bool>` field to `GatesSpec` — per-spec UAT override (D8). Overrides `[workflow] uat_enabled` when set.
- [x] 1.7 Tests: status serialization roundtrip, valid/invalid transitions, auto-promotion on gate pass, backward transition for rework, UAT field roundtrip

## 2. Workflow Engine (assay-core/workflow)

- [x] 2.1 Define `NextAction` enum in `assay-core/workflow`
- [x] 2.2 Implement `next_action(assay_dir) → Result<NextAction>` — reads milestones, specs, gate history. `ReviewSpec` is guidance, not enforcement (D7).
- [x] 2.3 Handle edge cases: no milestones, multiple InProgress milestones, quick milestones, specs without status
- [x] 2.4 Add strict_status check: when `[workflow] strict_status = true`, `cycle_advance` requires spec status >= `approved` before running gates (D7)
- [x] 2.5 Tests: each `NextAction` variant with fixture data, idempotency (pure function), early return for `Idle`, strict_status rejection of draft spec

## 3. Session Retention (assay-core/work_session)

- [x] 3.1 Add `[sessions]` config section to `assay-core/config` with `max_count` (default 100) and `max_age_days` (default 90)
- [x] 3.2 Implement `evict_sessions()` function: delete by count + age, skip sessions linked to InProgress milestones
- [x] 3.3 Call `evict_sessions()` lazily in `session_create` and `session_list`
- [x] 3.4 Tests: eviction by count, eviction by age, combined, active milestone protection

## 4. Plan Quick (assay-core + assay-cli)

- [x] 4.1 Add `quick: bool` field to `Milestone` struct with `#[serde(default)]`
- [x] 4.2 Implement `plan_quick(name, criteria)` in `assay-core/wizard` — creates milestone + chunk + spec with matching slugs, `quick: true`
- [ ] 4.3 Add `assay plan quick` CLI subcommand with interactive criteria collection
- [ ] 4.4 Update `milestone list` output to annotate quick milestones
- [x] 4.5 Tests: quick milestone creation, cycle_status/cycle_advance compatibility, quick flag persistence

## 5. Branch Isolation (assay-core/config + workflow)

- [x] 5.1 Add `[workflow]` config section with `auto_isolate` (`always | never | ask`, default `ask`), optional `protected_branches` list, `uat_enabled` (default `false`), and `strict_status` (default `false`) (D5, D7, D8)
- [x] 5.2 Implement `should_isolate(config, current_branch) → IsolationDecision` (enum: `Yes`, `No`, `Ask`). Call `detect_default_branch()` to dynamically supplement hardcoded list (D5/Q3).
- [x] 5.3 Default protected branch list: `["main", "master", "develop"]` + dynamic detection result. Config `protected_branches` overrides entire list when set.
- [ ] 5.4 Integrate isolation check into `plan_quick` and plan skill — prompt or auto-create worktree based on decision
- [x] 5.5 Tests: protected branch detection, custom list, dynamic detection of non-standard default (e.g., "trunk"), detection failure fallback, feature branch passthrough, config override

## 6. Smart Gate Routing (assay-core/gate)

- [x] 6.1 Implement `evaluate_routed(spec, working_dir, config)` that inspects criterion kinds and dispatches to Path 1/2/3 per criterion
- [x] 6.2 Handle mixed specs: run Path 1 criteria first, then Path 3 for AgentReport, merge results into unified `GateRunSummary`
- [x] 6.3 Skip pipeline-only criteria (`EventCount`, `NoToolErrors`) with informational note
- [x] 6.4 Add `[gate] agent_eval_mode` config option (`"auto"` | `"manual"`, default `"auto"`) to control AgentReport routing
- [x] 6.5 Tests: pure-command spec, pure-agent spec, mixed spec, pipeline-only skip, config override

## 7. Gate Evidence Rendering (assay-core)

- [x] 7.1 Add `render` module in `assay-core/gate` with functions: `render_terminal()`, `render_markdown_collapsed()`, `render_pr_body()`, `render_pr_check()`
- [x] 7.2 `render_terminal()`: 1-line summary with counts and duration
- [x] 7.3 `render_pr_check()`: per-criterion table with `<details>` collapsible evidence blocks
- [ ] 7.4 Update `pr_create` to post gate results as PR comment using `render_pr_check()`
- [x] 7.5 Tests: each renderer with fixture GateRunRecord, edge cases (no evidence, long output, all-pass, mixed)

## 8. Plugin Skills (plugins/claude-code + codex + opencode)

- [x] 8.1 Create `/assay:explore` skill — tiered context loading (D9): config summary + milestone list + spec index (~2K tokens). Full criteria on demand. Fresh project: config only + prompt.
- [x] 8.2 Create `/assay:focus` skill — calls `cycle_status` + `chunk_status` + `spec_get`, hides milestone for quick specs
- [x] 8.3 Create `/assay:check` skill — calls `evaluate_routed()`, then `next_action()` to suggest next step
- [x] 8.4 Create `/assay:ship` skill — verifies gates pass, calls `pr_create` with rendered evidence
- [x] 8.5 Update `/assay:plan` skill — add `quick` argument support, integrate branch isolation check
- [x] 8.6 Create separate SKILL.md files for deprecated names (D10): `status/` → focus content + deprecation notice, `next-chunk/` → focus content + deprecation notice, `gate-check/` → check content + deprecation notice. Remove after one version cycle.
- [x] 8.7 Update plugin CLAUDE.md with new skill table
- [x] 8.8 Port skill updates to codex and opencode plugins

## 9. CLI Updates (assay-cli)

- [x] 9.1 Add `assay plan quick` subcommand delegating to `plan_quick()`
- [ ] 9.2 Update `assay gate run` output to use `render_terminal()`
- [ ] 9.3 Update `assay pr create` to post gate comment using `render_pr_check()`
- [x] 9.4 Add `--status` filter flag to `assay spec list`

## 10. Integration & Verification

- [ ] 10.1 End-to-end test: `plan quick` → implement → `gate run` → auto-promote spec → `cycle_advance` → PR
- [ ] 10.2 End-to-end test: mixed-kind spec with Command + AgentReport criteria evaluated via smart routing
- [x] 10.3 Verify backward compatibility: existing specs without `status` field load and evaluate correctly
- [x] 10.4 Run `just ready` — full check suite passes
