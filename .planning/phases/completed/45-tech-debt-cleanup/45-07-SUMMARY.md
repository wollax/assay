# 45-07 Summary: CLI Cosmetic Sweep

## Status: Complete

## Issues Resolved (18)

### Code Fixes
- **format-relative-time-dedup**: Extracted shared `relative_from_secs` private helper; both `format_relative_time` and `format_relative_timestamp` delegate to it, eliminating duplicated threshold logic.
- **format-criteria-type-static-str**: Changed `format_criteria_type` return type from `String` to `&'static str`, eliminating heap allocations for static literals.
- **format-spec-not-found-vec-simplify**: Simplified the `shown` Vec construction in `assay-core/src/spec/mod.rs` using iterator chaining.
- **column-gap-invisible-value**: Added `// 2 spaces` trailing comment to `COLUMN_GAP` constant.
- **deterministic-results-variable-name**: Already resolved by a prior wave; issue moved to closed.
- **format-criteria-string-alloc**: Earlier duplicate of the static-str issue; moved to closed.

### Doc Improvements
- **stream-config-new-missing-doc**: Added doc comment to `StreamConfig::new()`.
- **stream-counters-failed-doc-unclear**: Clarified `StreamCounters::failed` doc from "(required enforcement)" to "required criteria".

### Worktree CLI Fixes
- **worktree-list-ignores-worktree-dir-flag**: Removed `--worktree-dir` from the `List` subcommand (flag was silently ignored since `list` uses git, not the filesystem).
- **worktree-cleanup-all-path**: Always use `entry.path` (canonical git path) in `cleanup --all`; removed the heuristic fallback to `worktree_dir.join(slug)`.
- **worktree-status-unwrap-or-false**: Replaced `unwrap_or(false)` with explicit pattern matching on `AssayError::WorktreeNotFound`; other errors are now propagated (single-spec) or warned (--all) instead of silently treated as clean.
- **worktree-list-params-unused-field**: Synchronized `WorktreeListParams.worktree_dir` schemars description with the Rust doc comment explaining it is currently unused.

### Test Improvements
- **format-spec-not-found-boundary-test**: Added `format_spec_not_found_ten_specs_no_truncation` test for the exact 10-item boundary.
- **gate-history-params-missing-name-test**: Added `test_gate_history_params_missing_name` test verifying serde error when `name` is absent.
- **test-unwrap-expect-messages**: Replaced bare `.err().unwrap()` with `.err().expect("...")` in `test_gate_report_params_invalid_passed_type` and `test_gate_run_params_invalid_include_evidence_type`.
- **format-command-error-test-weak-assert**: Tightened assertions from `contains("cargo")` to `contains("'cargo'")` in all three `format_command_error` tests.
- **format-toml-error-multiline-weak-assert**: Tightened assertion from `contains("line")` to `contains("line 2")` plus added caret assertion.
- **timeout-test-assertion-escape-hatch**: Removed `|| msg.contains("invalid value")` escape hatch from `test_gate_run_params_invalid_timeout_type`.

## Files Modified
- `crates/assay-cli/src/commands/mod.rs`
- `crates/assay-cli/src/commands/gate.rs`
- `crates/assay-cli/src/commands/worktree.rs`
- `crates/assay-core/src/spec/mod.rs`
- `crates/assay-core/src/gate/mod.rs`
- `crates/assay-core/src/config/mod.rs`
- `crates/assay-mcp/src/server.rs`

## Verification
`just ready` passes: fmt-check + lint + test (833 passed, 3 ignored) + deny.
