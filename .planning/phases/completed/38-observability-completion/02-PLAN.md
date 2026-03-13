---
phase: 38
plan: 2
wave: 1
depends_on: []
files_modified:
  - crates/assay-types/src/context.rs
  - crates/assay-core/src/context/tokens.rs
  - crates/assay-mcp/src/server.rs
autonomous: true
must_haves:
  truths:
    - "TokenEstimate has an optional growth_rate field that is omitted when not present"
    - "GrowthRate struct has avg_tokens_per_turn, estimated_turns_remaining, and turn_count fields"
    - "Growth rate is computed only when 5 or more non-sidechain assistant turns with usage data exist"
    - "Growth rate is absent (not zero) when fewer than 5 turns exist"
    - "Sidechain assistant entries are excluded from turn counting"
    - "estimate_tokens tool description mentions growth rate metrics"
  artifacts:
    - path: "crates/assay-types/src/context.rs"
      provides: "GrowthRate struct and growth_rate field on TokenEstimate"
    - path: "crates/assay-core/src/context/tokens.rs"
      provides: "collect_turn_tokens and compute_growth_rate functions"
  key_links:
    - from: "parse_session (full file read)"
      to: "collect_turn_tokens"
      via: "filters non-sidechain assistant entries with usage data"
    - from: "collect_turn_tokens"
      to: "compute_growth_rate"
      via: "passes turn token snapshots and context window size"
    - from: "compute_growth_rate"
      to: "TokenEstimate.growth_rate"
      via: "Option<GrowthRate> set on the estimate struct"
---

<objective>
Add growth rate metrics to the `estimate_tokens` MCP tool.

When 5 or more non-sidechain assistant turns exist, `estimate_tokens` returns `growth_rate` with avg_tokens_per_turn, estimated_turns_remaining, and turn_count. When fewer than 5 turns exist, the growth_rate field is absent entirely (not zero). This requires a full session parse instead of the current tail-only read.
</objective>

<context>
@crates/assay-types/src/context.rs (TokenEstimate struct at line 353, ContextHealth, UsageData)
@crates/assay-core/src/context/tokens.rs (estimate_tokens function, is_sidechain, extract_usage, constants)
@crates/assay-core/src/context/parser.rs (parse_session function signature)
@crates/assay-mcp/src/server.rs (estimate_tokens handler at line 1167, EstimateTokensParams)
@.planning/phases/pending/38-observability-completion/38-RESEARCH.md (Growth rate code examples, Turn counting, Pitfalls P1-P4)
</context>

<task type="auto">
  <name>Task 1: Add GrowthRate type and update TokenEstimate</name>
  <files>crates/assay-types/src/context.rs</files>
  <action>
  1. Add a `GrowthRate` struct before the `TokenEstimate` struct:
     ```rust
     /// Growth rate metrics for a session's context usage.
     ///
     /// Computed from non-sidechain assistant turns with usage data.
     /// Only available when 5 or more qualifying turns exist.
     #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
     pub struct GrowthRate {
         /// Average context tokens added per assistant turn.
         pub avg_tokens_per_turn: u64,
         /// Estimated assistant turns remaining before context window is full.
         pub estimated_turns_remaining: u64,
         /// Number of assistant turns used to compute these metrics.
         pub turn_count: u64,
     }
     ```

  2. Add `growth_rate` field to `TokenEstimate`:
     ```rust
     /// Growth rate metrics. Absent when fewer than 5 assistant turns exist.
     #[serde(skip_serializing_if = "Option::is_none")]
     pub growth_rate: Option<GrowthRate>,
     ```

  3. Update ALL existing construction sites of `TokenEstimate` in the codebase to include `growth_rate: None` so they compile. The only construction site is in `crates/assay-core/src/context/tokens.rs` in the `estimate_tokens` function — this will be updated in Task 2 to compute the actual value.
  </action>
  <verify>
  rtk cargo check -p assay-types
  </verify>
  <done>
  - GrowthRate struct exists with avg_tokens_per_turn, estimated_turns_remaining, turn_count
  - TokenEstimate has growth_rate: Option<GrowthRate> with skip_serializing_if
  - GrowthRate derives Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema
  - Crate compiles
  </done>
</task>

<task type="auto">
  <name>Task 2: Implement growth rate computation and integrate into estimate_tokens</name>
  <files>crates/assay-core/src/context/tokens.rs, crates/assay-mcp/src/server.rs</files>
  <action>
  **In `crates/assay-core/src/context/tokens.rs`:**

  1. Add import for `parse_session` and `GrowthRate`:
     ```rust
     use assay_types::context::GrowthRate;
     use super::parser::parse_session;
     ```

  2. Add a constant for the minimum turn threshold:
     ```rust
     /// Minimum number of assistant turns required to compute growth rate metrics.
     const MIN_TURNS_FOR_GROWTH_RATE: usize = 5;
     ```

  3. Add `collect_turn_tokens` function:
     ```rust
     /// Collect context_tokens from each non-sidechain assistant turn with usage data.
     ///
     /// Returns a Vec of cumulative context token counts, one per qualifying turn,
     /// in chronological order.
     fn collect_turn_tokens(path: &Path) -> crate::Result<Vec<u64>> {
         let (entries, _) = parse_session(path)?;
         let tokens: Vec<u64> = entries
             .iter()
             .filter(|e| !is_sidechain(&e.entry))
             .filter_map(|e| match &e.entry {
                 SessionEntry::Assistant(a) => {
                     a.message.as_ref()?.usage.as_ref().map(|u| u.context_tokens())
                 }
                 _ => None,
             })
             .collect();
         Ok(tokens)
     }
     ```

  4. Add `compute_growth_rate` function:
     ```rust
     /// Compute growth rate from turn token snapshots.
     ///
     /// Returns `None` when fewer than `MIN_TURNS_FOR_GROWTH_RATE` turns exist.
     /// Uses total context tokens divided by turn count for average growth,
     /// then estimates remaining turns from available context budget.
     fn compute_growth_rate(turn_tokens: &[u64], context_window: u64) -> Option<GrowthRate> {
         if turn_tokens.len() < MIN_TURNS_FOR_GROWTH_RATE {
             return None;
         }
         let turn_count = turn_tokens.len() as u64;
         let last = *turn_tokens.last()?;
         let avg = last / turn_count;
         let available = context_window.saturating_sub(SYSTEM_OVERHEAD_TOKENS);
         let remaining_tokens = available.saturating_sub(last);
         let remaining_turns = if avg > 0 { remaining_tokens / avg } else { 0 };

         Some(GrowthRate {
             avg_tokens_per_turn: avg,
             estimated_turns_remaining: remaining_turns,
             turn_count,
         })
     }
     ```

  5. Modify the `estimate_tokens` function to compute growth rate:
     - After computing the `TokenEstimate` fields (usage, health, etc.), call `collect_turn_tokens(path)?`
     - Then call `compute_growth_rate(&turn_tokens, context_window)`
     - Set the `growth_rate` field on the returned `TokenEstimate`

     The modified `estimate_tokens` should look like:
     ```rust
     pub fn estimate_tokens(path: &Path, session_id: &str) -> crate::Result<TokenEstimate> {
         let usage = quick_token_estimate(path)
             .map_err(|source| crate::AssayError::Io { ... })?
             .ok_or_else(|| crate::AssayError::SessionParse { ... })?;

         let context_window = DEFAULT_CONTEXT_WINDOW;
         let available = context_window.saturating_sub(SYSTEM_OVERHEAD_TOKENS);
         let context_tokens = usage.context_tokens();
         let pct = (context_tokens as f64 / available as f64) * 100.0;

         let health = if pct < 60.0 { ... };

         // Compute growth rate (requires full parse)
         let turn_tokens = collect_turn_tokens(path)?;
         let growth_rate = compute_growth_rate(&turn_tokens, context_window);

         Ok(TokenEstimate {
             session_id: session_id.to_string(),
             context_tokens,
             output_tokens: usage.output_tokens,
             context_window,
             context_utilization_pct: pct,
             health,
             growth_rate,
         })
     }
     ```

  6. Add tests in the existing `#[cfg(test)] mod tests` block:
     - `compute_growth_rate_returns_none_below_threshold`: verify None with 0, 1, 4 turns
     - `compute_growth_rate_returns_some_at_threshold`: verify Some with exactly 5 turns
     - `compute_growth_rate_calculates_correctly`: verify avg_tokens_per_turn, estimated_turns_remaining, turn_count with known inputs
     - `compute_growth_rate_saturates_when_full`: verify 0 remaining turns when context exceeds available
     - `compute_growth_rate_handles_zero_avg`: verify 0 remaining turns when avg is 0 (all turns had 0 tokens)
     - `collect_turn_tokens_filters_sidechains`: verify sidechain entries are excluded (use ParsedEntry test helpers already in the module)

  **In `crates/assay-mcp/src/server.rs`:**

  7. Update the `estimate_tokens` tool description to mention growth rate:
     Change the description to:
     ```
     "Estimate current token usage and context window health for a Claude Code session. \
      Returns context tokens, output tokens, utilization percentage, and a health indicator \
      (healthy/warning/critical). When 5+ assistant turns exist, includes growth_rate with \
      avg_tokens_per_turn, estimated_turns_remaining, and turn_count. \
      Omit session_id to estimate the most recent session for this project."
     ```
     Note: Remove "Fast: reads only the tail of the session file." since it now does a full parse for growth rate.
  </action>
  <verify>
  rtk cargo test -p assay-core
  rtk cargo test -p assay-mcp
  rtk cargo clippy --workspace -- -D warnings
  </verify>
  <done>
  - collect_turn_tokens parses full session and returns non-sidechain assistant turn token counts
  - compute_growth_rate returns None for fewer than 5 turns, Some(GrowthRate) for 5+
  - estimate_tokens includes growth_rate in the returned TokenEstimate
  - MCP tool description updated to mention growth rate metrics
  - Unit tests cover: threshold boundary, correct calculation, saturation, zero avg, sidechain filtering
  - All workspace tests pass, clippy clean
  </done>
</task>
