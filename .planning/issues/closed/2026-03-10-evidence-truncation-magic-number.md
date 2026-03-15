# Evidence Truncation Magic Number 200

## Description

The literal `200` is used twice for evidence truncation (once for evidence, once for reasoning) without a named constant. Additionally, evidence truncation uses `.len()` (byte count) while reasoning uses `.chars().take(200)` (scalar count), creating an inconsistency for non-ASCII content. Both sites should use a shared named constant and a consistent truncation strategy.

## File Reference

`crates/assay-cli/src/commands/gate.rs`

## Category

code
