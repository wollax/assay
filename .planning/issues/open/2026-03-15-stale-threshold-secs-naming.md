# `stale_threshold` Should Be Named `stale_threshold_secs`

## Description

`SessionsConfig` already uses `poll_interval_secs` and `recovery_window_secs` — both names include the `_secs` suffix to make the unit explicit. `stale_threshold` breaks this convention and leaves the unit ambiguous. Renaming it to `stale_threshold_secs` restores consistency and prevents misinterpretation (e.g., milliseconds vs. seconds).

This is a breaking change to the config schema and should be coordinated with a schema-version bump or migration note.

## File Reference

`crates/assay-types/src/lib.rs` — `SessionsConfig.stale_threshold`

## Category

naming / consistency
