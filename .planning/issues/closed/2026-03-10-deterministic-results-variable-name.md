# Misleading `deterministic_results` Variable Name

## Description

The `deterministic_results` variable name in `server.rs` is misleading; `results` or a more descriptive name would better convey intent without implying a specific ordering guarantee that may or may not be the actual concern.

## File Reference

`crates/assay-mcp/src/server.rs` (`deterministic_results`)

## Category

naming

## Severity

low
