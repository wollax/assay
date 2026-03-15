# Rename `enriched_error_display` to `format_enriched_error`

## Description

`enriched_error_display` in `gate/mod.rs` does not follow the `format_*` naming pattern used by `format_command_error` in the same module. Renaming it to `format_enriched_error` would make the module's API consistent.

## File Reference

`crates/assay-core/src/gate/mod.rs`

## Category

code
