# Plan 22-03 Summary: Content-Modification Strategies

## Status: COMPLETE

## What Was Done

Implemented four pruning strategies and wired all six `PruneStrategy` dispatch arms in `apply_strategy()`:

### thinking_blocks
- Filters `ContentBlock::Thinking` variants from assistant entries
- Uses `ParsedEntry::update_content()` to re-serialize with updated `raw_line`/`raw_bytes`
- Entries without thinking blocks pass through unchanged (no re-serialization overhead)

### metadata_strip
- Removes `System`, `FileHistorySnapshot`, `QueueOperation`, and `PrLink` entries entirely
- Line-deletion strategy (entries removed, not content-modified)

### tool_output_trim
- Truncates tool result string content exceeding 100 lines to first 20 + `[...N lines truncated...]` marker + last 20
- Processes each `tool_result` block independently within a user entry
- Uses `ParsedEntry::update_content()` for content-modified entries

### system_reminder_dedup
- Two-pass approach: reverse scan finds last occurrence of each unique `<system-reminder>` text, forward scan removes earlier duplicates
- Detects reminders in both assistant `ContentBlock::Text` and user JSON text blocks
- Line-deletion strategy for duplicates

### Dispatch Cleanup
- All six `PruneStrategy` match arms in `apply_strategy()` now route to real implementations
- Removed dead `StrategyResult::noop()` method (no longer needed)

## Test Coverage

- 30 tests across the four strategies (6 + 7 + 7 + 9 + 1 name overlap)
- 247 total tests in assay-core pass
- Clippy clean, formatting clean

## Commits

1. `test(22-03): add failing tests for thinking-blocks and metadata-strip strategies`
2. `feat(22-03): implement thinking-blocks and metadata-strip strategies`
3. `test(22-03): add failing tests for tool-output-trim strategy`
4. `feat(22-03): implement tool-output-trim strategy`
5. `test(22-03): add failing tests for system-reminder-dedup strategy`
6. `feat(22-03): implement system-reminder-dedup strategy and wire all dispatch arms`

## Files Modified

- `crates/assay-core/src/context/pruning/strategies/thinking_blocks.rs` (new)
- `crates/assay-core/src/context/pruning/strategies/metadata_strip.rs` (new)
- `crates/assay-core/src/context/pruning/strategies/tool_output_trim.rs` (new)
- `crates/assay-core/src/context/pruning/strategies/system_reminder_dedup.rs` (new)
- `crates/assay-core/src/context/pruning/strategies/mod.rs` (4 modules added)
- `crates/assay-core/src/context/pruning/strategy.rs` (all 6 arms wired, noop removed)

## Deviations

None. Plan executed as specified.
