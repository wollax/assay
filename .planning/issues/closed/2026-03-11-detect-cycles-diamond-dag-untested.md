# Test: diamond DAG (shared dependency) not tested

**Source:** Phase 37 PR review (test-reviewer)
**Area:** assay-core/spec
**File(s):** crates/assay-core/src/spec/validate.rs

## Description

The `Color::Black` branch in `detect_cycles` — which short-circuits traversal for nodes already fully processed — is only reachable when a node has multiple parents (a diamond-shaped dependency graph). No existing test exercises this path, so the shared-ancestor handling is unverified and a regression could allow cycles to go undetected in diamond graphs.

## Suggested Fix

Add a test with a diamond DAG (e.g., A→B, A→C, B→D, C→D) and assert that no cycle is reported. Also add a variant where the diamond is extended with an actual cycle to confirm detection still works.
