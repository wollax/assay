# Phase 22: Pruning Engine — Context

## Phase Scope

**Goal:** Composable, team-aware pruning strategies that safely reduce session bloat while preserving critical coordination messages.

**Requirements:**
- TPROT-05: Composable pruning strategies (progress-collapse, metadata-strip, thinking-blocks, tool-output-trim, stale-reads, system-reminder-dedup)
- TPROT-06: Team-aware pruning preserves coordination messages

**Depends on:** Phase 20 (session parser), Phase 21 (team extractor for protection)

**CLI surface:** `assay context prune` (grouped with existing `assay context diagnose/list`)

---

## Decisions

### Strategy Behavior & Boundaries

**progress-collapse:** Remove all progress tick messages entirely (delete JSONL lines). No collapsing to a representative message.

**stale-reads:** Remove all but the last read of each file path. The latest read has current content; earlier reads are definitionally stale.

**tool-output-trim:** Content-aware heuristics — strip extraneous information, keep important information. **Note: full content-aware heuristics may need a dedicated phase to develop. Initial implementation should use simpler heuristics (first/last N lines) with a clear extension point.**

**thinking-blocks:** Remove thinking blocks entirely. Rely on summaries at the end of thought processes. Do not truncate to partial summaries.

**metadata-strip:** Strip hook success messages, repeated git status blocks, and session start boilerplate. Preserve timing data and model info (small, useful for diagnostics).

**system-reminder-dedup:** Keep only the last occurrence of each repeated system reminder. The last is authoritative (content may have changed mid-session).

**Line deletion:** Strategies can delete entire JSONL lines (progress ticks, stale reads) or modify content within lines (truncation). Line count changes after pruning.

**Ambiguous content:** When a strategy encounters a message it can't cleanly categorize, apply partial pruning — strip what it can identify, leave the rest.

### Prescription Tiers

Prescriptions are named bundles of strategies. Each strategy receives the tier as a parameter for intensity-aware behavior.

| Tier | Strategies |
|------|-----------|
| **Gentle** | progress-collapse, system-reminder-dedup |
| **Standard** | gentle + metadata-strip, stale-reads |
| **Aggressive** | standard + thinking-blocks, tool-output-trim |

**Individual strategies:** Users can run individual strategies directly via `--strategy <name>`. Prescriptions are the default UX; individual strategies for power users.

**Intensity per strategy:** The prescription tier is passed into each strategy. Not every strategy uses it (progress-collapse is binary), but strategies like tool-output-trim and thinking-blocks can vary behavior by tier (e.g., trim to 50 lines at gentle, 20 at standard, 5 at aggressive).

**Savings reporting:** Report actual savings per strategy. No target ranges — results depend on session content.

### Composition Model

**Pipeline (sequential):** Each strategy is a pure function `Vec<ParsedEntry> -> Vec<ParsedEntry>`. Each strategy operates on the output of the previous one. Savings are tracked per-strategy via before/after delta.

**Ordering:** Prescription ordering determines execution order. Strategies that remove lines run before strategies that trim content.

### Dry-Run Output & Execution Flow

**Dry-run is default.** Output includes:
- Per-strategy summary table (lines removed, tokens saved, % reduction)
- 3 sample removals per strategy with "...and N more" suffix
- Aggregate totals at the bottom

**`--execute`:** Passing `--execute` is sufficient intent. No interactive confirmation prompt.

**Session targeting:** Requires explicit session path/ID. No auto-detection of current session.

### Backup & Restore

**Backup location:** `.assay/backups/` directory with timestamped filenames.

**Automatic backup:** Created before any `--execute` modification. No backup on dry-run.

**Retention limit:** Backups have a configurable retention limit (prevents unbounded disk usage from large session files).

**`--restore`:** Lists available backups for the session, user specifies which to restore. No "restore most recent" default.

### Team Message Protection

**Protected message types:** All `Task*` (Task, TaskCreate, TaskUpdate, TaskOutput, TaskGet, TaskList, TaskStop), all `Team*` (TeamCreate, TeamDelete), and `SendMessage`. Anything that crosses agent boundaries.

**Protection scope:** Entire JSONL line is protected. No partial trimming of lines containing coordination messages.

**Skip reporting:** Per-strategy report includes "protected: N lines skipped" count. Explains lower-than-expected savings.

**Always active:** Team protection always applies. No `--no-team-protect` flag. Zero cost when no team messages exist; prevents footgun if team members are added later.

---

## Deferred Ideas

- **Content-aware tool-output-trim heuristics** — Full semantic understanding of tool output (e.g., knowing which parts of a file read are relevant) may warrant its own phase. Initial implementation uses simpler line-based heuristics.

---

## Open Questions

None — all gray areas resolved.
