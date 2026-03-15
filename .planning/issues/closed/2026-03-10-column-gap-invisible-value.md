# `COLUMN_GAP` Value Is Invisible

## Description

`COLUMN_GAP` is defined as `"  "` (two spaces), which is impossible to audit at a glance. Add a trailing comment such as `// 2 spaces` to make the intent immediately clear without counting characters.

## File Reference

`crates/assay-cli/src/commands/mod.rs`

## Category

docs
