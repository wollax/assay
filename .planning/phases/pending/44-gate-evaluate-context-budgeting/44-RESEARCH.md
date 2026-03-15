# Phase 44: gate_evaluate Context Budgeting - Research

**Researched:** 2026-03-15
**Domain:** Token budgeting, diff truncation, gate_evaluate MCP handler
**Confidence:** HIGH — all findings derived from direct codebase inspection

---

## Summary

Phase 44 wires token-aware diff budgeting into the `gate_evaluate` MCP handler. The infrastructure is almost entirely in place: `budget_context` (Phase 39) already does the heavy lifting — it computes a token budget from a model window, handles passthrough for small content, and truncates the diff via the cupel pipeline when needed. What is missing is (a) calling `budget_context` in `gate_evaluate` instead of the current `truncate_diff` byte-budget approach, (b) extracting the truncated diff back from the result, and (c) capturing and surfacing truncation metadata.

All three concerns are self-contained changes to `crates/assay-mcp/src/server.rs` (the handler) and `crates/assay-types/src/gate_run.rs` (the `GateRunRecord` type). No new crates or dependencies are required.

**Primary recommendation:** Replace the `truncate_diff` call in `gate_evaluate`'s Step 3 with `budget_context` (or a purpose-built `budget_diff` helper that wraps `budget_context`), add an optional `DiffTruncation` struct to `GateRunRecord`, populate it when truncation occurred, and emit a `warnings` entry for every truncated evaluation.

---

## Current State (What Already Exists)

### `budget_context` in `crates/assay-core/src/context/budgeting.rs`

```rust
pub fn budget_context(
    system_prompt: &str,
    spec_body: &str,
    criteria_text: &str,
    diff: &str,
    model_window: u64,
) -> Result<Vec<String>, AssayError>
```

- Returns content in canonical order: `[system_prompt, spec_body, criteria_text, diff]`
- Passthrough optimization: if everything fits, returns immediately without pipeline overhead
- Pipeline path: diff is the primary truncation target (priority 50); system_prompt + criteria_text are pinned (always included); spec_body has priority 80
- `OUTPUT_RESERVE = 4_096`, `SAFETY_MARGIN_PERCENT = 5.0`
- `pub` — already exported from `assay-core::context`

### `context_window_for_model` in `crates/assay-core/src/context/tokens.rs`

```rust
pub fn context_window_for_model(_model: Option<&str>) -> u64 {
    DEFAULT_CONTEXT_WINDOW  // 200_000
}
```

Currently ignores the model argument and returns 200K for all models. This is correct for all current Claude models. This function is `pub` (no `pub(crate)` restriction mentioned for Phase 44).

**Important:** The decision from Phase 39 says "tokens module stays `pub(crate)`", but `context_window_for_model` itself is already `pub` — it's the module that is `pub(crate)`. Callers outside `context/` must use it via `super::tokens::context_window_for_model` or it must be re-exported. Currently it is NOT re-exported from `context/mod.rs`.

### Existing diff truncation in `gate_evaluate` (Step 3)

```rust
let diff = {
    match std::process::Command::new("git")
        .args(["diff", "HEAD"])
        .current_dir(&working_dir)
        .output()
    {
        Ok(output) if output.status.success() => {
            let raw = String::from_utf8_lossy(&output.stdout);
            let (truncated, _was_truncated, _original_bytes) =
                assay_core::gate::truncate_diff(&raw, DIFF_BUDGET_BYTES);
            truncated
        }
        // ... error handling
    }
};
```

`DIFF_BUDGET_BYTES = 32 * 1024` (32 KiB). This is a **byte budget**, not a token budget. The variables `_was_truncated` and `_original_bytes` are currently discarded.

### `truncate_diff` in `crates/assay-core/src/gate/mod.rs`

```rust
pub fn truncate_diff(raw: &str, budget: usize) -> (Option<String>, bool, Option<usize>)
```

Uses `truncate_head_tail` (head 33% / tail 67% split). No file-boundary awareness. This function can remain — it's used by `gate_run`.

### `GateRunRecord` (currently no truncation fields)

```rust
pub struct GateRunRecord {
    pub run_id: String,
    pub assay_version: String,
    pub timestamp: DateTime<Utc>,
    pub working_dir: Option<String>,
    pub summary: GateRunSummary,
}
```

Has `#[serde(deny_unknown_fields)]` — **adding new fields requires schema snapshot regeneration**.

### `GateEvaluateResponse` (currently no truncation fields)

```rust
struct GateEvaluateResponse {
    run_id: String,
    spec_name: String,
    summary: GateEvaluateSummary,
    results: Vec<EvaluateCriterionResult>,
    overall_passed: bool,
    evaluator_model: String,
    duration_ms: u64,
    warnings: Vec<String>,      // Phase 35 — already exists
    session_id: Option<String>,
}
```

---

## Architecture Patterns

### Pattern: Calling `budget_context` in `gate_evaluate`

`budget_context` takes the system prompt, spec body, criteria text, and raw diff together and returns them back in order after truncation. In `gate_evaluate`, these map to:

- `system_prompt` → result of `build_system_prompt()`
- `spec_body` → `description` (the spec's description text)
- `criteria_text` → the human-readable criteria listing built for the prompt
- `diff` → the raw git diff output
- `model_window` → `context_window_for_model(Some(&model))`

The simplest integration approach:

1. Capture raw diff from `git diff HEAD` (no truncation yet)
2. Build system prompt + criteria text (as currently done in Steps 4 and 5)
3. Call `budget_context(system_prompt, spec_body, criteria_text, &raw_diff, model_window)` — this returns the ordered parts with the diff already truncated if needed
4. Extract the diff from the returned vec (it's last, or absent if fully dropped)
5. Detect truncation: compare returned diff length to raw diff length (or check if diff is absent)
6. Pass the (possibly-truncated) diff into `build_evaluator_prompt`

**Alternative approach:** A thinner helper `budget_diff_only` that just computes available diff tokens and truncates. This avoids re-assembling the full prompt from parts and keeps the existing step ordering cleaner. However, it duplicates the budget arithmetic from `budget_context`. Given that `budget_context` is stable and tested, using it directly is preferred.

### Pattern: Determining truncation metadata

After calling `budget_context`:

```rust
let raw_diff_bytes = raw_diff.len();
let budgeted = budget_context(&system_prompt, &spec_body, &criteria_text, &raw_diff, model_window)?;
// diff is last in the canonical order if present
let truncated_diff = budgeted.last().filter(|s| s.starts_with("diff") || !s.is_empty());
// Detect truncation by comparing lengths
let diff_was_truncated = truncated_diff.map(|d| d.len()) != Some(raw_diff.len());
```

In practice, the check is: if the raw diff was non-empty and the returned diff is shorter (or absent), truncation occurred.

### Pattern: File list extraction from diff

The CONTEXT.md requires that truncation metadata include **which files were included and which were omitted**. Git diff output contains `diff --git a/<file> b/<file>` headers. A simple parser can extract file paths from the raw diff and from the truncated diff to compute the two sets.

The `diff --git a/path b/path` header format is stable. A function that extracts file paths from a diff string:

```rust
fn extract_diff_files(diff: &str) -> Vec<String> {
    diff.lines()
        .filter(|l| l.starts_with("diff --git "))
        .filter_map(|l| l.split(" b/").nth(1))
        .map(|s| s.to_string())
        .collect()
}
```

Files in raw diff but not in truncated diff = omitted. Files in both = included.

### Pattern: `DiffTruncation` struct vs flat fields on `GateRunRecord`

Given that `GateRunRecord` has `deny_unknown_fields`, a new optional struct is cleaner than multiple flat fields. It can be skipped when absent:

```rust
/// Truncation metadata for the diff passed to the evaluator.
/// Present only when truncation occurred (diff exceeded token budget).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DiffTruncation {
    /// Token budget available for the diff.
    pub budget_tokens: u64,
    /// Estimated token count of the original diff.
    pub original_tokens: u64,
    /// Byte size of the original diff.
    pub original_bytes: usize,
    /// Byte size of the diff after truncation.
    pub truncated_bytes: usize,
    /// Files included in the truncated diff.
    pub included_files: Vec<String>,
    /// Files omitted from the truncated diff (present in original, absent in truncated).
    pub omitted_files: Vec<String>,
}
```

Added to `GateRunRecord`:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub diff_truncation: Option<DiffTruncation>,
```

### Pattern: `GateEvaluateResponse` truncation visibility

The MCP response already has a `warnings` field. When truncation occurs:
- Add a warning: `"diff truncated: <N> files omitted to fit <X>K token budget (kept <M> files)"`
- Optionally include `diff_truncation` in the response struct (mirrors `GateRunRecord`)

The CONTEXT.md says metadata is included in the MCP response **only when truncation occurred**, and **truncation always triggers a warning**. This means:
- `GateEvaluateResponse` should also gain an optional `diff_truncation` field
- The warning is emitted unconditionally when truncation occurs

### Pattern: Graceful fallback on budget failure

When `budget_context` returns an error:

```rust
let (effective_diff, diff_truncation) = match budget_context(...) {
    Ok(parts) => (extract_diff(&parts, &raw_diff), compute_metadata(&parts, &raw_diff, model_window)),
    Err(e) => {
        tracing::warn!("diff budgeting failed: {e} — using full diff");
        warnings.push(format!("diff budget computation failed: {e} — full diff passed to evaluator"));
        (Some(raw_diff.clone()), None)
    }
};
```

This matches the CONTEXT.md requirement: "If budgeting fails, graceful fallback — pass full diff through and log a warning."

### Pattern: Model window lookup

```rust
let model_window = assay_core::context::tokens::context_window_for_model(Some(&model));
```

The `tokens` module is `pub(crate)` inside `assay-core`, but `context_window_for_model` is `pub`. Since `gate_evaluate` is in `assay-mcp` (a separate crate), the function must be re-exported from `assay-core`. Currently it is NOT. One of two approaches:

1. Add `pub use tokens::context_window_for_model;` to `context/mod.rs`
2. Use `estimate_tokens_from_bytes` (already exported) and hard-code 200K in the server

Option 1 is cleaner and consistent with Phase 39's decision that "tokens module stays `pub(crate)`" — the module visibility is `pub(crate)`, but individual functions can be selectively re-exported.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Token budget calculation | Custom byte/token arithmetic | `budget_context` | Already tested, handles passthrough + pipeline path |
| Diff truncation algorithm | New truncation logic | `budget_context` (cupel pipeline) | Handles priority-based truncation with pinned items |
| Model context window lookup | Hard-coded table | `context_window_for_model` | Already exists, all current models return 200K |
| Token estimation | New byte-to-token ratio | `estimate_tokens_from_bytes` (3.7 bytes/token) | Already calibrated |

---

## Common Pitfalls

### Pitfall 1: `GateRunRecord` deny_unknown_fields schema snapshot

**What goes wrong:** Adding `diff_truncation` to `GateRunRecord` without regenerating schema snapshots causes test failures in `crates/assay-types/tests/schema_snapshots.rs`.

**How to avoid:** After adding the field, run `cargo test` — if snapshots fail, run the generate-schemas example to update them.

**Warning signs:** Test `schema_snapshots` failures mentioning `gate-run-record`.

### Pitfall 2: Extracting diff from `budget_context` result

**What goes wrong:** Assuming the diff is always last in the returned vec. If the diff is empty or fully dropped, it won't appear at all. If spec body or criteria were also truncated (unusual), ordering holds but lengths change.

**How to avoid:** Detect the diff by comparing content against the raw diff string, or by knowing the canonical order (system_prompt, spec_body, criteria_text, diff) and checking whether the last element matches the diff prefix.

**Simpler approach:** Call `budget_context` purely to detect truncation and get the truncated diff string. The full prompt is then built normally via `build_evaluator_prompt` using the truncated diff.

### Pitfall 3: Criteria text for budgeting vs prompt building

**What goes wrong:** The `build_evaluator_prompt` function builds criteria text internally. If you pass the same criteria text to both `budget_context` and `build_evaluator_prompt`, you're computing the same string twice.

**How to avoid:** Either: (a) extract criteria text building into a helper that both `budget_context` and `build_evaluator_prompt` can share, or (b) accept the double computation (it's cheap for typical criterion counts).

### Pitfall 4: Zero or tiny model window

**What goes wrong:** If `model` is a short alias like `"sonnet"` rather than a full model ID, `context_window_for_model` still returns 200K (correct behavior), but a future model with a different window would be missed.

**How to avoid:** Current behavior is correct. Log the model string and window size at debug level so it's visible.

### Pitfall 5: budget_context changes prompt order

**What goes wrong:** If you reassemble the full evaluator prompt from `budget_context` output (treating the Vec<String> as prompt sections), the prompt structure changes. `build_evaluator_prompt` already knows how to structure the prompt — don't replace it.

**How to avoid:** Use `budget_context` only to get the (possibly truncated) diff. Pass that truncated diff to `build_evaluator_prompt` as before.

---

## Code Examples

### Extracting truncated diff from `budget_context` output

```rust
// HIGH confidence — based on budget_context's documented canonical order:
// [system_prompt, spec_body, criteria_text, diff]
fn extract_truncated_diff(
    budgeted: &[String],
    raw_diff: &str,
) -> Option<String> {
    if raw_diff.is_empty() {
        return None;
    }
    // Diff is last in the returned vec (if present)
    budgeted.last().cloned().filter(|s| !s.is_empty())
}

fn was_truncated(budgeted: &[String], raw_diff: &str) -> bool {
    if raw_diff.is_empty() {
        return false;
    }
    match budgeted.last() {
        Some(d) => d.len() < raw_diff.len(),
        None => true, // diff was fully dropped
    }
}
```

### Extracting file lists from diff

```rust
// HIGH confidence — git diff format is stable
fn extract_diff_files(diff: &str) -> Vec<String> {
    diff.lines()
        .filter(|l| l.starts_with("diff --git "))
        .filter_map(|l| l.split(" b/").nth(1).map(str::to_string))
        .collect()
}
```

### Integration point in gate_evaluate (conceptual)

The change is localized to Step 3 of `gate_evaluate`:

```rust
// ── Step 3: Compute git diff with token budget ─────────────────
let (system_prompt, schema_json) = (
    assay_core::evaluator::build_system_prompt(),
    assay_core::evaluator::evaluator_schema_json(),
);
let criteria_text = build_criteria_text(&criteria);  // extracted helper or inline
let model_window = assay_core::context::tokens::context_window_for_model(Some(&model));

let (diff, diff_truncation_info) = match git_diff_raw(&working_dir) {
    Some(raw) => {
        match assay_core::context::budget_context(
            &system_prompt, &description, &criteria_text, &raw, model_window
        ) {
            Ok(budgeted) => {
                let truncated_diff = extract_truncated_diff(&budgeted, &raw);
                let meta = if was_truncated(&budgeted, &raw) {
                    Some(build_truncation_metadata(&raw, truncated_diff.as_deref(), model_window))
                } else {
                    None
                };
                (truncated_diff, meta)
            }
            Err(e) => {
                warnings.push(format!("diff budget failed: {e} — using full diff"));
                tracing::warn!("budget_context failed: {e}");
                (Some(raw), None)
            }
        }
    }
    None => (None, None),
};

// diff_truncation_info is then used to populate GateRunRecord and GateEvaluateResponse
```

---

## Type Changes Required

### `crates/assay-types/src/gate_run.rs`

Add `DiffTruncation` struct and optional field on `GateRunRecord`:

```rust
/// Truncation metadata when the diff was truncated to fit the token budget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DiffTruncation {
    pub budget_tokens: u64,
    pub original_tokens: u64,
    pub original_bytes: usize,
    pub truncated_bytes: usize,
    pub included_files: Vec<String>,
    pub omitted_files: Vec<String>,
}

// In GateRunRecord:
#[serde(default, skip_serializing_if = "Option::is_none")]
pub diff_truncation: Option<DiffTruncation>,
```

Note: `GateRunRecord` has `#[serde(deny_unknown_fields)]` — this is an additive field change with `skip_serializing_if`, which is safe for new records. Old records (without the field) deserialize correctly because of `#[serde(default)]`.

### `crates/assay-mcp/src/server.rs`

Add truncation fields to `GateEvaluateResponse`:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
diff_truncation: Option<DiffTruncationSummary>,
```

(The response may use a lighter summary type — file lists + key sizes — rather than the full `DiffTruncation` from `assay-types`.)

### `crates/assay-core/src/context/mod.rs`

Re-export `context_window_for_model`:

```rust
pub use tokens::context_window_for_model;
```

---

## Open Questions

1. **Criteria text extraction:** `build_evaluator_prompt` builds criteria text internally. To pass the same text to `budget_context`, it must be extracted into a shared helper or computed separately. The cleanest option: add a `build_criteria_text(criteria: &[Criterion]) -> String` helper in `evaluator.rs` (already `pub`). This is low-risk, but it's a new public function.

   - Recommendation: add the helper, keep it `pub(crate)` or `pub` for testing.

2. **Token estimate for truncation metadata:** `budget_tokens` field requires computing the available diff token budget. This comes from `budget_context`'s internal arithmetic: `(model_window - OUTPUT_RESERVE) * 0.95` minus overhead for system_prompt, spec_body, criteria_text. This isn't exposed by `budget_context`. Options:
   - Expose a `compute_diff_budget()` function from `budgeting.rs`
   - Estimate it separately using `estimate_tokens_from_bytes` on the overhead strings
   - Include only `original_tokens` and `truncated_tokens` without `budget_tokens` (simpler)

   Recommendation: skip `budget_tokens` in the initial implementation and include only `original_bytes`, `truncated_bytes`, `included_files`, `omitted_files`. Budget arithmetic details are implementation details.

3. **`DiffTruncation` location:** Should it live in `assay-types/src/gate_run.rs` (alongside `GateRunRecord`) or a new file? Given it's only used by `GateRunRecord`, co-locating in `gate_run.rs` is appropriate.

---

## Sources

### Primary (HIGH confidence)
- Direct codebase inspection of `crates/assay-core/src/context/budgeting.rs` — `budget_context` API, constants, behavior
- Direct codebase inspection of `crates/assay-core/src/context/tokens.rs` — `context_window_for_model`, `estimate_tokens_from_bytes`
- Direct codebase inspection of `crates/assay-mcp/src/server.rs` — `gate_evaluate` handler, Steps 1-10, current truncation approach
- Direct codebase inspection of `crates/assay-types/src/gate_run.rs` — `GateRunRecord` structure, `deny_unknown_fields`
- Direct codebase inspection of `crates/assay-core/src/gate/mod.rs` — `truncate_diff`, `truncate_head_tail`

### Secondary (MEDIUM confidence)
- `.planning/phases/completed/39-context-engine-integration/` — prior decisions about `budget_context` API and cupel integration patterns

---

## Metadata

**Confidence breakdown:**
- Current codebase state: HIGH — direct inspection
- Integration approach: HIGH — flow is clear from reading the handler
- Type design (`DiffTruncation`): HIGH — consistent with existing patterns
- File list extraction from diff: HIGH — git diff format is stable

**Research date:** 2026-03-15
**Valid until:** 2026-04-15 (stable domain — no fast-moving dependencies)
