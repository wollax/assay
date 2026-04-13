# Phase 70: Wire Resolution + Preconditions into Gate Pipeline - Research

**Researched:** 2026-04-13
**Domain:** Rust gate evaluation pipeline integration — connecting existing compose/precondition APIs to CLI, MCP, and TUI surfaces
**Confidence:** HIGH

## Summary

All domain logic required for this phase exists and is tested. `compose::resolve()`, `check_preconditions()`, and `evaluate_all_resolved()` are complete, tested, and production-ready. `GateEvalOutcome` with its `Evaluated` / `PreconditionFailed` variants is defined in `assay-types`. The only work is connecting these pieces to the callers that currently bypass them.

The three surfaces each have a distinct pattern: CLI has `handle_gate_run()` and `handle_gate_run_all()` in `crates/assay-cli/src/commands/gate.rs`; MCP has `gate_run()` in `crates/assay-mcp/src/server.rs` (async, uses `spawn_blocking`); TUI has no direct gate evaluation path (no gate run calls were found in the TUI crate — only gate history display). History persistence via `assay_core::history::save_run()` takes a `GateRunSummary`, not `GateEvalOutcome` — the caller must pattern-match the outcome before saving.

**Primary recommendation:** Add the three-step pipeline (resolve → check_preconditions → evaluate_all_resolved) in-place at each call site for `SpecEntry::Directory`. Legacy specs continue to call `evaluate_all()` unchanged. No new helper functions wrapping all three steps — each surface has surface-specific behavior between them.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Gate run output format:**
- Inline source tag after criterion name in CLI streaming output: e.g. `✔ lint-check [Parent: base-gate]`
- Tags appear for inherited/library criteria
- JSON mode: per-criterion `source` field already exists from Phase 66
- CLI exit code 2 for precondition failures — distinct from 0 (pass) and 1 (gate failed)
- PreconditionFailed runs saved to history with distinct outcome type — full audit trail
- Same retention/pruning rules as normal evaluation runs (PreconditionFailed entries count toward max_history)
- Legacy specs (SpecEntry::Legacy) cannot participate in composition — resolution only applies to SpecEntry::Directory
- Gate history is format-agnostic (keyed by spec name)

### Claude's Discretion
- Source annotation visibility for Own criteria (show tag or omit)
- JSON composition summary metadata shape
- CLI summary line composition counts
- Precondition failure display format (CLI)
- MCP PreconditionFailed response shape (error vs successful with outcome)
- Legacy spec resolution behavior (silent skip vs debug log)
- Whether to always resolve directory specs or check fields first
- `last_gate_passed()` semantics for PreconditionFailed entries
- gate_history detail level for blocked entries
- TUI gate run display adaptations for source annotations and precondition failures

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| INHR-02 | Extended gate inherits parent criteria with own-wins merge semantics | `compose::resolve()` implements this; callers must call it before `evaluate_all_resolved()` |
| INHR-04 | Gate run output shows per-criterion source annotation (parent vs own) | `CriterionResult.source: Option<CriterionSource>` already exists; `stream_criterion()` needs tag display; `format_gate_response()` needs `source` field on `CriterionSummary` |
| CLIB-02 | User can reference criteria libraries via `include` field in gate definitions | `compose::resolve()` handles `include`; same integration point as INHR-02 |
| PREC-01 | User can define `[preconditions].requires` — gate skipped unless named spec's last gate run passed | `check_preconditions()` + `last_gate_passed()` implement this; callers must invoke before evaluation |
| PREC-02 | User can define `[preconditions].commands` — shell commands that must succeed before gate evaluation | `check_preconditions()` handles this; same integration point as PREC-01 |
| PREC-03 | Precondition failures produce distinct `PreconditionFailed` result (blocked != failed) | `GateEvalOutcome::PreconditionFailed(PreconditionStatus)` exists; callers must use it for CLI exit code 2 and distinct history record |
</phase_requirements>

## Standard Stack

### Core APIs (all pre-existing in this codebase)

| Function | Location | Purpose | Signature |
|----------|----------|---------|-----------|
| `compose::resolve()` | `assay-core/src/spec/compose.rs:243` | Expand extends + include into ResolvedGate | `resolve(gate, gate_slug, load_gate, load_library) -> Result<ResolvedGate>` |
| `check_preconditions()` | `assay-core/src/gate/mod.rs:300` | Evaluate requires + commands, return status | `check_preconditions(preconditions, last_gate_passed_fn, working_dir, cli_timeout, config_timeout) -> PreconditionStatus` |
| `evaluate_all_resolved()` | `assay-core/src/gate/mod.rs:371` | Evaluate resolved criteria with source annotations | `evaluate_all_resolved(spec_name, resolved, gate_section, working_dir, cli_timeout, config_timeout) -> GateRunSummary` |
| `PreconditionStatus::all_passed()` | — | Check if all preconditions passed | Must check `requires.iter().all(|r| r.passed) && commands.iter().all(|c| c.passed)` (no method, must be done inline or added) |
| `assay_core::history::last_gate_passed()` | `assay-core/src/history/mod.rs:229` | Query whether last run passed | `last_gate_passed(assay_dir, spec_name) -> Option<bool>` |
| `assay_core::spec::load_gates()` | `assay-core/src/spec/mod.rs:401` | Load a GatesSpec by path | Used to build `load_gate` closure |
| `compose::load_library_by_slug()` | `assay-core/src/spec/compose.rs:168` | Load a library by slug | Used to build `load_library` closure |
| `assay_core::history::save_run()` | `assay-core/src/history/mod.rs:72` | Persist GateRunSummary as GateRunRecord | Takes `GateRunSummary` — caller must extract from `GateEvalOutcome::Evaluated` |

### Key Types

| Type | Location | Role in this phase |
|------|----------|-------------------|
| `GateEvalOutcome` | `assay-types/src/gate_run.rs:134` | Return type from new pipeline; `Evaluated(GateRunSummary)` or `PreconditionFailed(PreconditionStatus)` |
| `ResolvedGate` | `assay-types/src/resolved_gate.rs:50` | Output of `compose::resolve()`; contains `Vec<ResolvedCriterion>` |
| `ResolvedCriterion` | `assay-types/src/resolved_gate.rs:35` | Criterion + source; fed to `evaluate_all_resolved()` |
| `CriterionSource` | `assay-types/src/resolved_gate.rs:15` | `Own`, `Parent { gate_slug }`, `Library { slug }` |
| `PreconditionStatus` | `assay-types/src/precondition.rs:43` | Output of `check_preconditions()`; contains `requires: Vec<RequireStatus>`, `commands: Vec<CommandStatus>` |
| `SpecPreconditions` | `assay-types/src/precondition.rs:18` | Input type; sourced from `gates.preconditions: Option<SpecPreconditions>` |
| `CriterionSummary` | `assay-mcp/src/server.rs:1001` | MCP per-criterion response struct; needs `source` field added |
| `GateRunResponse` | `assay-mcp/src/server.rs:854` | MCP aggregate response struct; may need `precondition_status` field |

## Architecture Patterns

### Pipeline Flow (per call site)

```
SpecEntry::Legacy { spec, .. }
  → evaluate_all(spec, ...)    [unchanged]

SpecEntry::Directory { gates, slug, .. }
  → Step 1: compose::resolve(gates, slug, load_gate_fn, load_library_fn)
      → Ok(resolved_gate)  → continue
      → Err(e)             → surface error (CLI: bail, MCP: CallToolResult::error)

  → Step 2: if gates.preconditions.is_some()
        → check_preconditions(preconditions, |s| last_gate_passed(assay_dir, s), working_dir, cli_timeout, config_timeout)
        → if !status.all_passed()
              → GateEvalOutcome::PreconditionFailed(status)  [surface-specific handling]

  → Step 3: evaluate_all_resolved(slug, &resolved_gate.criteria, gates.gate.as_ref(), working_dir, cli_timeout, config_timeout)
      → GateEvalOutcome::Evaluated(summary)
```

### Closure Construction Pattern

Both `load_gate` and `load_library` closures are built from paths already available at each call site:

```rust
// Source: assay-core/src/spec/compose.rs (compose::resolve signature)
let specs_dir_clone = specs_dir.clone();
let load_gate = |slug: &str| -> assay_core::error::Result<GatesSpec> {
    let path = specs_dir_clone.join(slug).join("gates.toml");
    assay_core::spec::load_gates(&path)
};

let assay_dir_clone = assay_dir.clone();
let load_library = |slug: &str| -> assay_core::error::Result<CriteriaLibrary> {
    assay_core::spec::compose::load_library_by_slug(&assay_dir_clone, slug)
};
```

### All-Passed Check Pattern

`PreconditionStatus` has no `all_passed()` method. The check must be done inline:

```rust
// Inferred from assay-types/src/precondition.rs
let blocked = status.requires.iter().any(|r| !r.passed)
    || status.commands.iter().any(|c| !c.passed);
if blocked {
    // return GateEvalOutcome::PreconditionFailed(status)
}
```

### History Recording — PreconditionFailed

The CONTEXT.md requires saving PreconditionFailed runs to history. However, `assay_core::history::save_run()` takes a `GateRunSummary`, not a `GateEvalOutcome`. The plan must decide on one of:

**Option A (recommended):** Add a new `save_precondition_failed_run()` function to `assay-core/src/history/mod.rs` that takes `spec_name: &str`, `status: &PreconditionStatus`, and records a `GateRunRecord` with a synthetic summary (0 passed/failed/skipped, enforcement summary zeroed, a special marker). This avoids straining `GateRunRecord`'s `deny_unknown_fields`.

**Option B:** Extend `GateRunRecord` to hold `Option<PreconditionStatus>` and update the serde schema. This touches the history format and requires schema snapshot updates.

**Key constraint:** `GateRunRecord` uses `#[serde(deny_unknown_fields)]` — adding a new field with `#[serde(default, skip_serializing_if = "Option::is_none")]` is backward-compatible for reading old records, but would require schema snapshot updates (`crates/assay-types/tests/schema_snapshots.rs`).

Option A is cleaner since PreconditionFailed has no criteria results to store — a synthetic summary is honest.

### `last_gate_passed()` and PreconditionFailed

Current implementation (`history/mod.rs:229`) loads the latest `GateRunRecord` and checks `record.summary.enforcement.required_failed == 0`. A synthetic PreconditionFailed summary with `required_failed = 0` would cause `last_gate_passed()` to return `Some(true)` for a blocked run — wrong semantics.

To disambiguate, the PreconditionFailed record needs a marker. Recommended approach: store a synthetic summary with a sentinel value (e.g., a specific `spec_name` suffix, or a dedicated boolean flag) or use a simple convention: `enforcement.required_failed = usize::MAX` is not viable. The cleanest solution is a new optional field on `GateRunRecord`:

```rust
// In assay-types/src/gate_run.rs
#[serde(default, skip_serializing_if = "Option::is_none")]
pub precondition_blocked: Option<bool>,
```

This lets `last_gate_passed()` check: if `precondition_blocked == Some(true)` → return `Some(false)`. Old records without the field read as `None` → backward-compatible.

### CLI Exit Code Pattern

Existing pattern (gate.rs):
```rust
fn gate_exit_code(counters: &StreamCounters) -> i32 {
    if counters.gate_blocked() { 1 } else { 0 }
}
```

New pattern needed: exit code 2 when `GateEvalOutcome::PreconditionFailed`. Must be plumbed through `handle_gate_run()` return value.

### MCP Response Shape for PreconditionFailed

`format_gate_response()` takes a `GateRunSummary`. For precondition failure, the caller bypasses this function entirely and returns a response with a different shape. Two options:

**Option A (recommended):** Return `CallToolResult::success` with a `GateRunResponse`-like struct that has `outcome: "precondition_failed"` and `precondition_status: PreconditionStatus`. This follows the existing pattern (gate_run returns structured JSON on success).

**Option B:** Return `CallToolResult::error`. This matches the "error" response pattern already used for bad spec names etc., but conflates "spec not found" errors with "spec blocked" semantics — not ideal for agent callers who need to distinguish.

### TUI Surface

No gate evaluation code was found in `crates/assay-tui/`. The TUI reads history (`detail_run: Option<GateRunRecord>`) and displays it, but does not call `evaluate_all_gates()` directly. The TUI surface integration is limited to:
1. Displaying PreconditionFailed records from history (if a field is added to `GateRunRecord`)
2. Detail view adaptations in `app.rs` for `detail_run` when the record is a PreconditionFailed entry

The CONTEXT.md lists the TUI as an integration surface but the TUI likely delegates gate runs to the CLI/MCP layer. The TUI surface work may be limited to display adaptations for the new record shape.

### Anti-Patterns to Avoid
- **Wrapping all three steps in a single helper:** Each surface has distinct behavior between steps (CLI streams, MCP has agent session path). Inline the pipeline at each call site.
- **Changing `evaluate_all_gates()` signature:** Keep the old function intact; it serves the pipeline path (`assay-core/src/pipeline.rs:1153`) which does not need preconditions (pipeline manages its own lifecycle).
- **Using `GateEvalOutcome` as a history format:** `GateEvalOutcome` is explicitly documented as "in-memory return type only." History uses `GateRunRecord`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead |
|---------|-------------|-------------|
| Criteria inheritance merge | Custom merge algorithm | `compose::resolve()` — already handles own-wins semantics with reverse-dedup |
| Precondition evaluation | Custom shell execution | `check_preconditions()` — handles requires + commands, all-evaluated semantics |
| Source annotation threading | Manual field on summary | `evaluate_all_resolved()` — accepts `ResolvedCriterion` slice, threads source through |
| Library loading | Path manipulation + TOML parse | `compose::load_library_by_slug()` — handles path construction and parsing |
| Gate loading | Direct TOML parse | `assay_core::spec::load_gates()` — includes validation |

**Key insight:** All domain logic was built in Phases 65-66. This phase is pure integration.

## Common Pitfalls

### Pitfall 1: Calling `evaluate_all_gates()` from pipeline.rs
**What goes wrong:** `assay-core/src/pipeline.rs:1153` also calls `evaluate_all_gates_with_events()`. If the planner adds resolution/precondition logic there too, the pipeline gate evaluation gets precondition-gated, which is wrong — the pipeline manages its own session lifecycle and does not use preconditions.
**How to avoid:** Only modify the three explicitly listed integration points: `gate.rs` (CLI), `server.rs` (MCP), and `app.rs` (TUI display). Leave `pipeline.rs` unchanged.

### Pitfall 2: `GateRunRecord` deny_unknown_fields
**What goes wrong:** `GateRunRecord` has `#[serde(deny_unknown_fields)]`. Adding a new field is backward-compat for reading (with `#[serde(default)]`) but will fail schema snapshot tests in `crates/assay-types/tests/schema_snapshots.rs` unless snapshots are updated.
**How to avoid:** If adding `precondition_blocked: Option<bool>` to `GateRunRecord`, update the schema snapshot file.

### Pitfall 3: `PreconditionStatus` has no `all_passed()` method
**What goes wrong:** Assuming a helper method exists when it doesn't. Compiler error at integration site.
**How to avoid:** Implement the check inline, or add the method to `PreconditionStatus` in `assay-types` as part of this phase. Adding a method is low-risk and makes intent clear.

### Pitfall 4: streaming mode in CLI saves a synthetic summary
**What goes wrong:** The streaming path in CLI (`handle_gate_run()` non-JSON mode) already uses `streaming_summary()` which creates a minimal `GateRunSummary` without per-criterion results. The resolution path in streaming mode must do the same — criteria are streamed to stdout as they execute, and the final save uses counters, not `evaluate_all_resolved()` output.
**How to avoid:** In streaming mode: call `compose::resolve()` to get `resolved`, stream each `resolved.criteria` item via a modified `stream_criterion()` that accepts `Option<CriterionSource>`, track counters, then call `streaming_summary()` for history. JSON mode can call `evaluate_all_resolved()` directly and save the full `GateRunSummary`.

### Pitfall 5: `stream_criterion()` signature does not accept `CriterionSource`
**What goes wrong:** `stream_criterion()` takes a `&Criterion`, not a `&ResolvedCriterion`. Adding source tag display requires either a signature change or a wrapper.
**How to avoid:** Add `source: Option<&CriterionSource>` parameter to `stream_criterion()`. This is a small, contained change in `gate.rs`.

### Pitfall 6: `CriterionSummary` in MCP has no `source` field
**What goes wrong:** `format_gate_response()` maps `CriterionResult` to `CriterionSummary`. `CriterionResult.source` already carries the annotation from `evaluate_all_resolved()`, but `CriterionSummary` has no `source` field — the annotation is lost.
**How to avoid:** Add `source: Option<String>` (or structured type) to `CriterionSummary` and populate it in `format_gate_response()`.

## Code Examples

### compose::resolve() closure construction pattern

```rust
// Source: assay-core/src/spec/compose.rs:243 (resolve signature)
// The closures needed at each call site:

let specs_dir_for_resolve = specs_dir.clone();
let assay_dir_for_resolve = assay_dir.clone();

let resolved_gate = assay_core::spec::compose::resolve(
    gates,
    slug,
    |parent_slug| {
        let path = specs_dir_for_resolve.join(parent_slug).join("gates.toml");
        assay_core::spec::load_gates(&path)
    },
    |lib_slug| {
        assay_core::spec::compose::load_library_by_slug(&assay_dir_for_resolve, lib_slug)
    },
)?;
```

### check_preconditions() invocation pattern

```rust
// Source: assay-core/src/gate/mod.rs:300 (check_preconditions signature)
// preconditions comes from: gates.preconditions.as_ref()

if let Some(preconditions) = &gates.preconditions {
    let assay_dir_for_prec = assay_dir.clone();
    let status = assay_core::gate::check_preconditions(
        preconditions,
        move |spec_slug| assay_core::history::last_gate_passed(&assay_dir_for_prec, spec_slug),
        working_dir,
        cli_timeout,
        config_timeout,
    );
    let blocked = status.requires.iter().any(|r| !r.passed)
        || status.commands.iter().any(|c| !c.passed);
    if blocked {
        // Return GateEvalOutcome::PreconditionFailed(status)
        // Surface-specific: CLI prints blocked message and returns exit code 2
        // MCP: returns success with outcome="precondition_failed" field
    }
}
```

### evaluate_all_resolved() invocation pattern

```rust
// Source: assay-core/src/gate/mod.rs:371
let summary = assay_core::gate::evaluate_all_resolved(
    slug,
    &resolved_gate.criteria,
    gates.gate.as_ref(),
    working_dir,
    cli_timeout,
    config_timeout,
);
// summary.results[i].source is Some(CriterionSource::Own/Parent/Library)
```

### CLI streaming with source annotation

```rust
// Proposed: stream_criterion with source parameter (in gate.rs)
// eprint!("{cr}  {label} {} {} ... running", criterion.name, source_tag(source));
fn source_tag(source: Option<&CriterionSource>) -> &str {
    match source {
        Some(CriterionSource::Parent { gate_slug }) => format!("[Parent: {gate_slug}]"),
        Some(CriterionSource::Library { slug })     => format!("[Library: {slug}]"),
        Some(CriterionSource::Own) | None           => String::new(),
    }
}
```

## State of the Art

| Old Approach | Current Approach | Notes |
|--------------|-----------------|-------|
| `evaluate_all_gates()` ignores extends/include | `compose::resolve()` + `evaluate_all_resolved()` | This phase wires them together |
| No precondition checking | `check_preconditions()` exists | This phase calls it before evaluation |
| Exit codes: 0/1 | Exit codes: 0/1/2 | 2 = precondition blocked (new this phase) |
| `CriterionResult.source` always `None` for CLI | `CriterionResult.source` populated via `evaluate_all_resolved()` | This phase makes it visible in output |

## Open Questions

1. **`PreconditionFailed` history record format**
   - What we know: `GateRunRecord` uses `deny_unknown_fields`; `save_run()` takes `GateRunSummary`
   - What's unclear: whether to add `precondition_blocked: Option<bool>` to `GateRunRecord` (requires schema snapshot update) or use a synthetic summary with sentinel
   - Recommendation: Add `precondition_blocked: Option<bool>` to `GateRunRecord` with `#[serde(default, skip_serializing_if = "Option::is_none")]`; update schema snapshot; update `last_gate_passed()` to return `Some(false)` when `precondition_blocked == Some(true)`

2. **TUI surface extent**
   - What we know: No gate evaluation calls exist in `assay-tui/`; TUI reads `detail_run: Option<GateRunRecord>`
   - What's unclear: Whether TUI needs any changes at all, or just benefits from the richer records written by CLI/MCP
   - Recommendation: TUI changes are limited to `ChunkDetail` display in `app.rs` — if `detail_run` record has `precondition_blocked = Some(true)`, display a "blocked" indicator rather than pass/fail counts. This is optional polish.

3. **Whether to always resolve or check fields first**
   - What we know: Claude's discretion; resolution is cheap (2 file reads for extends + include)
   - Recommendation: Always call `compose::resolve()` for `SpecEntry::Directory` specs. Resolution is idempotent and cheap. Skip only on error, not conditionally based on field presence.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (standard Rust, no pytest/jest) |
| Config file | none — workspace Cargo.toml |
| Quick run command | `just test` or `cargo test -p assay-core -p assay-cli -p assay-mcp` |
| Full suite command | `just ready` (fmt-check + lint + test + deny) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| INHR-02 | `gate run` on spec with `extends` evaluates parent+own criteria with own-wins | integration | `cargo test -p assay-cli -- gate_run_extends` | Wave 0 |
| INHR-04 | CLI streaming output shows `[Parent: base-gate]` tag for inherited criteria | unit | `cargo test -p assay-cli -- stream_criterion_source_tag` | Wave 0 |
| INHR-04 | JSON output has `source` field populated in `CriterionResult` | unit | `cargo test -p assay-cli -- gate_run_json_source_annotation` | Wave 0 |
| CLIB-02 | `gate run` on spec with `include` evaluates library criteria | integration | `cargo test -p assay-cli -- gate_run_include_library` | Wave 0 |
| PREC-01 | `gate run` with `requires` referencing no-history spec → `PreconditionFailed` | integration | `cargo test -p assay-cli -- gate_run_precondition_requires_blocked` | Wave 0 |
| PREC-02 | `gate run` with failing command precondition → `PreconditionFailed` | integration | `cargo test -p assay-cli -- gate_run_precondition_command_blocked` | Wave 0 |
| PREC-03 | CLI exit code 2 on `PreconditionFailed` | integration | `cargo test -p assay-cli -- gate_run_exit_code_precondition_failed` | Wave 0 |
| PREC-03 | MCP `gate_run` returns precondition_failed outcome | integration | `cargo test -p assay-mcp -- gate_run_precondition_failed_mcp` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p assay-core -p assay-cli -p assay-mcp -- 2>&1 | grep -E "test result|FAILED"`
- **Per wave merge:** `just test`
- **Phase gate:** `just ready` (full suite including fmt, lint, deny) before `/kata:verify-work`

### Wave 0 Gaps

All test files listed above need to be created. The integration test pattern in this codebase places tests within the same source file as the implementation (see `assay-cli/src/commands/gate.rs` — no separate test files). Tests for CLI integration go in `crates/assay-cli/src/commands/gate.rs` under `#[cfg(test)]`. Tests for MCP integration go in `crates/assay-mcp/src/server.rs` under `#[cfg(test)]`.

No framework installation needed — cargo test is already configured.

## Sources

### Primary (HIGH confidence)

- Directly read source: `crates/assay-core/src/spec/compose.rs` (lines 243-356) — `resolve()` signature, closure interfaces, cycle detection
- Directly read source: `crates/assay-core/src/gate/mod.rs` (lines 246-394) — `evaluate_all_gates()`, `check_preconditions()`, `evaluate_all_resolved()` signatures and implementations
- Directly read source: `crates/assay-types/src/gate_run.rs` — `GateEvalOutcome`, `GateRunRecord`, `CriterionResult.source` field, `deny_unknown_fields` annotation
- Directly read source: `crates/assay-types/src/precondition.rs` — `PreconditionStatus`, `SpecPreconditions`, `RequireStatus`, `CommandStatus`
- Directly read source: `crates/assay-types/src/resolved_gate.rs` — `ResolvedGate`, `ResolvedCriterion`, `CriterionSource`
- Directly read source: `crates/assay-cli/src/commands/gate.rs` (lines 503-626) — `handle_gate_run()`, `handle_gate_run_all()`, `stream_criterion()`, `save_run_record()` implementations
- Directly read source: `crates/assay-mcp/src/server.rs` (lines 1644-1711, 5130-5220) — `gate_run()` handler, `format_gate_response()`, `GateRunResponse`, `CriterionSummary` struct
- Directly read source: `crates/assay-core/src/history/mod.rs` (lines 72-234) — `save_run()`, `last_gate_passed()` implementations
- Directly read source: `crates/assay-core/src/pipeline.rs` (lines 1140-1165) — `evaluate_all_gates_with_events()` call that must NOT be changed

### Secondary (MEDIUM confidence)

- Grep results confirming TUI has no direct gate evaluation calls (no matches for `evaluate_all_gates`, `check_preconditions`, `compose::resolve` in `crates/assay-tui/`)
- `crates/assay-types/tests/schema_snapshots.rs` confirmed to contain `GateRunRecord` schema (line 136 refs `GateEvalContext` but `GateRunRecord` has `inventory::submit!` registration)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all APIs read from source, signatures confirmed
- Architecture: HIGH — call sites located, patterns confirmed from working code
- Pitfalls: HIGH — based on direct code reading, not speculation
- History recording shape: MEDIUM — the `GateRunRecord.precondition_blocked` approach is recommended but requires planner to decide on schema change scope

**Research date:** 2026-04-13
**Valid until:** 2026-05-13 (stable codebase, no external dependencies)
