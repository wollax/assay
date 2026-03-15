# Duplicated `SpecEntry` Criteria Extraction in `gate.rs`

## Description

Match blocks that extract `gate_section` and `criteria` from `SpecEntry` appear at least twice in `gate.rs`. This logic should be consolidated into a helper function to reduce duplication and centralise any future changes to the extraction pattern.

## File Reference

`crates/assay-cli/src/commands/gate.rs`

## Category

code
