# Assert Both-Sides Ellipsis in `truncate_source_line_long_center`

## Description

`truncate_source_line_long_center` in `config/mod.rs` tests does not assert that ellipsis appears on both sides of the truncated string. The test should assert both `starts_with("...")` and `ends_with("...")` to fully verify centre-truncation behaviour.

## File Reference

`crates/assay-core/src/config/mod.rs` (tests — `truncate_source_line_long_center`)

## Category

tests
