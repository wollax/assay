# History --limit allows 0 (shows nothing)

**Area:** cli
**Severity:** suggestion
**Source:** Phase 15 PR review

## Description

`GateCommand::History` allows `limit: usize` of 0, which would show an empty table. Consider using `NonZeroUsize` or adding a clap value parser with `value_parser!(usize).range(1..)` to reject it at parse time.

**File:** `crates/assay-cli/src/main.rs`
