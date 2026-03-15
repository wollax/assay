# Module Doc Hardcodes "Seventeen Tools" — Count Will Drift

## Description

The module-level doc comment in `server.rs` states "seventeen tools". This number is hardcoded prose and will silently become wrong every time a tool is added or removed. Either remove the count from the doc comment, replace it with a link to the tool list, or derive the count from a constant so the doc stays accurate.

## File Reference

`crates/assay-mcp/src/server.rs` — module doc comment

## Category

documentation / maintainability
