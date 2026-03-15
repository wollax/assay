# Add Transposition Case to Levenshtein Tests

## Description

Levenshtein tests in `spec/mod.rs` do not cover transpositions. A test for "ab" → "ba" (expected cost 2) should be added to verify that transpositions are not mistakenly treated as single-edit operations.

## File Reference

`crates/assay-core/src/spec/mod.rs` (tests)

## Category

tests
