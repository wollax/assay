# Levenshtein Should Collect `b.chars()` Upfront

## Description

The Levenshtein implementation in `spec/mod.rs` calls `b.chars()` on every outer loop iteration. Collecting into a `Vec<char>` before the loop would avoid repeated iteration over the string.

## File Reference

`crates/assay-core/src/spec/mod.rs`

## Category

code
