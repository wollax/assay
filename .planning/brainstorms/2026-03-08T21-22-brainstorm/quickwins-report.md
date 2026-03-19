# Quick Wins Report — Assay v0.3.0

*Explorer: explorer-quickwins | Challenger: challenger-quickwins*
*Date: 2026-03-08*
*Debate rounds: 3*

---

## Executive Summary

Seven initial proposals were explored, pressure-tested through three rounds of debate, and refined to six actionable quick wins. One proposal was dropped (Issue Triage — already completed in Phase 25), one was reclassified from new feature to targeted improvement (`assay doctor` → error message improvements), and all surviving proposals were rescoped based on challenger critique.

**Total estimated effort:** ~8 days
**Issues addressed:** ~40-50 of 106 open issues
**Key theme:** Batch-close mechanical debt, improve agent DX, polish existing features.

---

## Approved Proposals (Priority Order)

### 1. CLI Correctness Sprint

**Priority:** P1 — Highest confidence, lowest risk
**Effort:** 1.5 days
**Issues resolved:** ~8-10 CLI issues

**Scope:**
- **Wave 1 (correctness):**
  - Fix `NO_COLOR` to use `var_os().is_none()` instead of `var().is_err()` (handles non-UTF-8 env vars per no-color.org spec)
  - Consolidate gate help text duplication (top-level + subcommand)
  - Remove enforcement check duplication in CLI
  - Fix spec show color branch duplication
- **Wave 2 (cosmetic, if time permits):**
  - Add `StreamCounters` doc comments
  - Add `tally()` and `gate_blocked()` helper methods
  - Rename `failed` → `failing` field (verified CLI-internal only, never serialized)
  - Add `StreamConfig` doc comments
  - Extract command separator constant

**Risks:** Low. Each fix is independent and testable. Help text changes need manual review to ensure completeness. Wave 2 changes are optional stretch goals.

**Debate outcome:** Originally scoped as 14 issues in 2 days. Challenger correctly identified the need to prioritize correctness over cosmetics. Split into waves.

---

### 2. MCP Parameter Validation

**Priority:** P2 — High impact for agent DX
**Effort:** 1-1.5 days
**Issues resolved:** 2-3 MCP issues

**Scope:**
- Add parameter structure validation to all 8 MCP tools with specific, actionable error messages
- Include "Available specs: ..." in spec-not-found errors (exact list, no fuzzy matching)
- Move validation logic to `assay-core` (keep MCP server as thin adapter)
- No new dependencies (no `strsim` — substring/prefix matching only if needed)
- Focus on the three most common failure modes:
  1. Missing required parameters
  2. Invalid parameter types
  3. Spec name not found

**Risks:** Low. MCP server has well-defined tool schemas. Validation in `assay-core` is architecturally clean. Agents already get spec names from `spec_list`, so name typos are rare — parameter structure errors are the real pain point.

**Debate outcome:** Originally included fuzzy matching via `strsim` dependency. Challenger argued agents rarely typo names (they copy from `spec_list`), and the project's `cargo-deny` policy favors minimal deps. Refocused on parameter structure validation.

---

### 3. Types Hygiene Tier A

**Priority:** P3 — Quick batch close, clean foundation
**Effort:** 0.5 days
**Issues resolved:** ~10-12 assay-types issues

**Scope (zero-risk additive changes only):**
- Add `Eq` derives alongside `PartialEq` on all types without float fields
- Implement `Display` for `Enforcement`, `GateKind`, and other enums
- Add missing doc comments on public types and fields
- Derive `Default` where semantically appropriate

**Explicitly deferred (Tier B — requires compatibility analysis):**
- `#[serde(default)]` on optional fields (could change deserialization of stored history)
- `#[deny(unknown_fields)]` additions (breaks forward compatibility)
- `#[non_exhaustive]` additions (API surface change)

**Risks:** Minimal. All changes are purely additive. Existing 493 tests provide safety net. Tier B deferred until stored history format compatibility is reviewed.

**Debate outcome:** Originally batched all 18 types issues together. Challenger identified that serde behavioral changes are compatibility decisions, not mechanical fixes. Split into zero-risk (Tier A) and needs-analysis (Tier B).

---

### 4. Gate/Spec Error Message Improvement

**Priority:** P4 — Better than `assay doctor`, focused scope
**Effort:** 1 day
**Issues resolved:** 3-4 issues (error ergonomics, spec parse errors, gate command validation)

**Scope (top 3 error scenarios by frequency):**
1. **Command not found during gate run** — When a gate criterion's command fails with `ENOENT`, show: "Command 'cargo-nextest' not found. Is it installed and in your PATH?" instead of a generic IO error
2. **Spec name not found** — Show available spec names: "Spec 'auth-flw' not found. Available specs: auth-flow, data-pipeline"
3. **Invalid spec TOML** — Show file path, line number, and the specific parse error instead of a generic "failed to parse spec"

**Not in scope:** New `assay doctor` command (deferred — new feature, not a quick win). `assay spec validate --all` could be a 2-hour stretch goal wrapping existing validation.

**Risks:** Low. Improving existing error paths, not adding new ones. Need to ensure error messages are concise enough for both human and agent consumers.

**Debate outcome:** Originally proposed as `assay doctor` — a new diagnostic command. Challenger correctly identified this as a new feature masquerading as a quick win. Replaced with targeted error message improvements in existing commands, which addresses the same user pain with less scope.

---

### 5. Gate Output Truncation Lite

**Priority:** P5 — Highest impact for agents, but larger scope
**Effort:** 3-4 days
**Issues resolved:** Phase 7 streaming capture (partial), output detail enum

**Scope:**
- **Pre-task (required):** Measure actual gate output sizes from existing history to validate the default limit. Check `original_bytes` field in stored `GateResult` records. Run representative gates (`cargo test`, `cargo clippy`, `just ready`) and measure output.
- Replace `Command::output()` with `Command::spawn()` + `BufReader` with byte counting
- Capture head (first N bytes) + tail (last N bytes) with `[truncated: X bytes omitted]` marker
- Use existing `truncated` and `original_bytes` fields on `GateResult` (schema already supports this)
- Default limit TBD based on measurement (starting hypothesis: 32KB / ~8K tokens)
- No configuration plumbing in v0.3.0 — hardcoded default only
- Handle UTF-8 boundary correctly (don't split multi-byte sequences)
- Independent stdout/stderr budgets

**Risks:** Medium. Changes the gate command execution model (`output()` → `spawn()`). Touches error handling and timeout behavior. Could break snapshot tests. Head+tail truncation may hide mid-stream errors in some cases, but is strictly better than no truncation. **Default limit must be validated against real data** — if too low, users see constant "[truncated]" warnings; if too high, doesn't solve the token waste problem.

**Debate outcome:** Originally scoped at 2-3 days. Challenger identified underscoping: UTF-8 handling, independent stdout/stderr budgets, and the need to validate the 32KB default against actual data. Revised to 3-4 days. Challenger initially suggested tail-only truncation; explorer pushed back (head+tail is ~10 extra lines of code for a much better heuristic), and challenger conceded.

---

### 6. Guard Daemon PID fsync

**Priority:** P6 — Tiny correctness fix
**Effort:** 0.25 days
**Issues resolved:** 1 issue (guard-pid-no-fsync)

**Scope:**
- Add `fsync()` after PID file write to ensure the PID is durably stored before the daemon assumes it's recorded
- Prevents race condition where daemon crashes after write but before OS flushes to disk

**Explicitly deferred:**
- Circuit breaker reset logic — requires a design decision between two approaches:
  - (a) Add `assay context guard reset` CLI command for manual reset
  - (b) Daemon catches the trip and enters sleep-then-retry instead of exiting
  - This should be documented as a design issue, not just coded
- All other guard daemon cosmetic improvements (Debug derives, Display impls, tracing instrumentation) — low priority given limited daemon usage

**Risks:** Minimal. Single-line `fsync()` call. Well-understood correctness pattern.

**Debate outcome:** Originally proposed as "Guard Daemon Hardening Batch" (15 issues, 2 days). Challenger questioned whether hardening a Unix-only daemon with limited users is "high impact." Narrowed to the single correctness fix that matters. Circuit breaker reset split into a separate design discussion.

---

## Dropped Proposals

### ~~Issue Triage & Consolidation~~
**Reason:** Phase 25 (completed Mar 7) already performed comprehensive issue triage. TRIAGE-SUMMARY.md exists with priority tiers, 17 issues were closed, and remaining 106 are categorized. This work was already done.

### ~~`assay doctor` Command~~
**Reason:** Reclassified as a new feature, not a quick win. Replaced with targeted error message improvements (#4 above) that address the same user pain with much less scope. A minimal `assay spec validate --all` could be added as a stretch goal in the error messages work.

---

## Recommended Execution Order

```
Week 1:
  #3 Types Hygiene A     (0.5d) ─── clears foundation, unblocks nothing but feels good
  #1 CLI Correctness     (1.5d) ─── highest confidence, ships fast
  #6 Guard PID fsync     (0.25d) ── tiny, ship with CLI PR or standalone
  #4 Error Messages      (1d)   ─── builds on CLI work momentum

Week 2:
  #2 MCP Validation      (1.5d) ─── agent DX improvement
  #5 Output Truncation   (3-4d) ─── largest item, benefits from earlier work settling
```

Types Hygiene and Guard fsync are small enough to pair with other work or ship as independent micro-PRs. CLI Correctness and Error Messages build on each other (both touch CLI error paths). MCP Validation and Output Truncation are independent and can be parallelized if multiple contributors are available.

---

## Success Metrics

- Open issue count drops from 106 to ~75-80 (25-30% reduction)
- All CLI correctness bugs fixed (NO_COLOR, help duplication)
- Agent retry rate drops due to better MCP error messages
- Gate output is bounded, preventing context window blowouts
- `just ready` passes after each proposal is completed
