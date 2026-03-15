# `dfs` invariant-violation `continue` is silent — should emit warning diagnostic

**Area:** assay-core/spec/validate.rs
**Severity:** suggestion
**Source:** PR review (Phase 37)

## Description

When `dfs` encounters a node that violates an internal invariant (e.g. a node that should have been coloured but is not in the colour map), it silently `continue`s without emitting any diagnostic. This means the invariant violation is invisible to the caller and to the user, making bugs very hard to diagnose.

## Suggested Fix

Replace the silent `continue` with an emitted `Warning` or `Error` diagnostic describing the invariant violation, so that unexpected states are surfaced rather than swallowed.
