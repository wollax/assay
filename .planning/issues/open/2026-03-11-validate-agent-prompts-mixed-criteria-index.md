# Test: mixed-kind criteria index correctness

**Source:** Phase 37 PR review (test-reviewer)
**Area:** assay-core/spec
**File(s):** crates/assay-core/src/spec/validate.rs

## Description

`validate_agent_prompts` is only tested with a single-element criteria list, so the `index` field embedded in diagnostic locations has never been verified for a non-first `AgentReport` entry in a mixed-kind criteria list. If the index counting logic is off-by-one or resets incorrectly when criteria kinds change, no existing test would catch it.

## Suggested Fix

Add a test with multiple criteria of mixed kinds (e.g., at least one `Pass/Fail` criterion followed by an `AgentReport` criterion) and assert that the diagnostic location index correctly reflects the item's position in the full criteria list.
