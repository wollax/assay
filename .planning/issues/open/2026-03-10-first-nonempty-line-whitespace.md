# `first_nonempty_line` Returns Untrimmed Lines

## Description

`first_nonempty_line` does not trim whitespace before checking whether a line is empty. A line containing only spaces or tabs would pass the non-empty check and produce a whitespace-only failure reason, which would be misleading to callers.

## File Reference

`crates/assay-mcp/src/server.rs` (`first_nonempty_line`)

## Category

correctness

## Severity

medium
