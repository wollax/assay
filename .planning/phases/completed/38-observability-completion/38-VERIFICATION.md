---
phase: 38
status: passed
---

# Phase 38: Observability Completion — Verification

**Status: passed**

---

## Plan 01 — OBS-03: `spec_get` resolved config with timeout precedence

### Must-Have Checks

**✓ "spec_get accepts an optional resolve: bool parameter (default false)"**

`server.rs` lines 44–57:
```rust
pub struct SpecGetParams {
    pub name: String,
    #[schemars(description = "Include resolved configuration (effective timeouts, working_dir validation)")]
    #[serde(default)]
    pub resolve: bool,
}
```
`#[serde(default)]` ensures `false` when omitted. Confirmed by test at line 3732–3737.

---

**✓ "When resolve is true, the response includes a resolved object with timeout cascade and working_dir validation"**

`server.rs` lines 594–613: when `params.0.resolve` is true, a `resolved_block` is built with `"timeout"` and `"working_dir"` sub-objects, then inserted into both Legacy and Directory response JSON (lines 623–625 and 639–641). Integration tests at lines 3793–3845 and 3848–3894 confirm the block appears in the output.

---

**✓ "Timeout cascade always has the same shape: effective, spec (null), config, default fields"**

`server.rs` lines 598–604:
```rust
"timeout": {
    "effective": effective_timeout,
    "spec": serde_json::Value::Null,
    "config": config_timeout,
    "default": 300
}
```
`effective`, `spec`, `config`, `default` are always present. `spec` is hardcoded `Null`. Test at line 3832–3839 verifies all four fields.

---

**✓ "working_dir validation includes path, exists, and accessible fields"**

`server.rs` lines 605–609:
```rust
"working_dir": {
    "path": working_dir.to_string_lossy(),
    "exists": working_dir.exists(),
    "accessible": working_dir.is_dir()
}
```
Test at lines 3841–3845 and 3927–3933 verify `path` (string), `exists` (bool), `accessible` (bool).

---

**✓ "When resolve is false or omitted, no resolved block appears in the response"**

`server.rs` lines 611–613: when `params.0.resolve` is false, `resolved_block` is `None` and the conditional insert is skipped. Integration test at lines 3753–3791 asserts `json.get("resolved").is_none()`.

---

**✓ "Config tier is null when no [gates] section exists"**

`server.rs` line 595: `config_timeout = config.gates.as_ref().map(|g| g.default_timeout)` — returns `None` when no `[gates]` section, which serializes as JSON `null`. Test at line 3836–3838 asserts `timeout["config"].is_null()` in a project with no `[gates]` section.

---

## Plan 02 — OBS-04: Growth rate metrics in `estimate_tokens`

### Must-Have Checks

**✓ "TokenEstimate has an optional growth_rate field that is omitted when not present"**

`context.rs` lines 366–383:
```rust
pub struct TokenEstimate {
    ...
    #[serde(skip_serializing_if = "Option::is_none")]
    pub growth_rate: Option<GrowthRate>,
}
```
`skip_serializing_if` ensures the field is absent (not `null`) when `None`.

---

**✓ "GrowthRate struct has avg_tokens_per_turn, estimated_turns_remaining, and turn_count fields"**

`context.rs` lines 354–362:
```rust
pub struct GrowthRate {
    pub avg_tokens_per_turn: u64,
    pub estimated_turns_remaining: u64,
    pub turn_count: u64,
}
```
All three required fields present, all typed `u64`.

---

**✓ "Growth rate is computed only when 5 or more non-sidechain assistant turns with usage data exist"**

`tokens.rs` line 19: `const MIN_TURNS_FOR_GROWTH_RATE: usize = 5;`

`tokens.rs` lines 137–139:
```rust
fn compute_growth_rate(turn_tokens: &[u64], context_window: u64) -> Option<GrowthRate> {
    if turn_tokens.len() < MIN_TURNS_FOR_GROWTH_RATE {
        return None;
    }
```

`collect_turn_tokens` (lines 117–130) filters only non-sidechain assistant entries with usage data before passing to `compute_growth_rate`.

---

**✓ "Growth rate is absent (not zero) when fewer than 5 turns exist"**

`tokens.rs` lines 311–317 (test `compute_growth_rate_returns_none_below_threshold`):
```rust
assert!(compute_growth_rate(&[], DEFAULT_CONTEXT_WINDOW).is_none());
assert!(compute_growth_rate(&[1000], DEFAULT_CONTEXT_WINDOW).is_none());
assert!(compute_growth_rate(&[1000, 2000, 3000, 4000], DEFAULT_CONTEXT_WINDOW).is_none());
```
Returns `None` (absent) for 0, 1, and 4 turns — not `Some(0)`.

---

**✓ "Sidechain assistant entries are excluded from turn counting"**

`tokens.rs` lines 119–128: `collect_turn_tokens` filters `.filter(|e| !is_sidechain(&e.entry))` before collecting. Integration test `collect_turn_tokens_filters_sidechains` (lines 360–423) writes a mixed session file with one sidechain entry and asserts `tokens.len() == 2` (the two non-sidechain entries only).

---

**✓ "estimate_tokens tool description mentions growth rate metrics"**

`server.rs` lines 1195–1200:
```
"Estimate current token usage and context window health for a Claude Code session. \
Returns context tokens, output tokens, utilization percentage, and a health indicator \
(healthy/warning/critical). When 5+ assistant turns exist, includes growth_rate with \
avg_tokens_per_turn, estimated_turns_remaining, and turn_count. \
Omit session_id to estimate the most recent session for this project."
```
Explicitly mentions `growth_rate`, `avg_tokens_per_turn`, `estimated_turns_remaining`, `turn_count`, and the 5-turn threshold.

---

## Overall Verdict

**All 12 must-haves are satisfied.** The implementation in:
- `crates/assay-mcp/src/server.rs` (SpecGetParams, spec_get handler, estimate_tokens description)
- `crates/assay-types/src/context.rs` (GrowthRate, TokenEstimate)
- `crates/assay-core/src/context/tokens.rs` (collect_turn_tokens, compute_growth_rate, estimate_tokens)

...matches every requirement in both Plan 01 and Plan 02. Tests cover the key behaviors including boundary conditions (exactly 4 turns returns None, exactly 5 returns Some), sidechain exclusion, and all response field shapes.
