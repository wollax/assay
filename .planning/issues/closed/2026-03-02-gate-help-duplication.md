---
title: Reduce duplication in gate command help examples
area: assay-cli
priority: low
source: Phase 9 PR review
---

## Problem

The Gate top-level `after_long_help` and the `GateCommand::Run` `after_long_help` have near-identical example sections. If a second gate subcommand is added in the future, the top-level examples may become stale or incomplete. This duplication creates a maintenance burden and a potential source of inconsistency.

## Solution

Consolidate the help examples into a single location (likely the `GateCommand::Run` subcommand) and update the top-level help to reference or delegate to the subcommand's examples. Alternatively, define a shared help text constant that is used by both locations.
