# Double-reverse for display_ids in handle_gate_history

**Area:** cli
**Severity:** suggestion
**Source:** Phase 15 PR review

## Description

`handle_gate_history()` uses a double-reverse iterator chain to take the last N items while preserving order. A simpler slice-based approach would be clearer:

```rust
let skip = ids.len().saturating_sub(limit);
let display_ids: Vec<&str> = ids[skip..].iter().map(|s| s.as_str()).collect();
```

**File:** `crates/assay-cli/src/main.rs`
