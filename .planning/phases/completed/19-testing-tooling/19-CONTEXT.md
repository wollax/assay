# Phase 19 Context: Testing & Tooling

## Overview

Phase 19 fills test coverage gaps across all v0.2.0 phases, tightens cargo-deny policies, and creates a dogfooding spec where Assay gates itself. This is the final hardening phase before the Cozempic-inspired features (phases 20-23).

## Decisions

### 1. MCP Handler Test Strategy

**Decision:** Both direct handler tests AND protocol-level (JSON-RPC) tests.

- **Direct handler tests:** Unit tests inside `crates/assay-mcp/src/server.rs` via `#[cfg(test)] mod tests`. Call handler functions directly with constructed arguments.
- **Protocol-level tests:** Integration tests in `crates/assay-mcp/tests/`. Send JSON-RPC requests through the MCP server and verify serialized responses.
- **File system:** Tempdir-based integration tests with real `.assay/` directory structures. No mock abstraction layer — Assay doesn't have one and adding one is scope creep.
- **Session state:** Multi-step lifecycle tests for agent gates (create session via gate_run → report via gate_report → finalize via gate_finalize). Tests exercise the actual `Arc<Mutex<HashMap>>` session store.
- **Coverage bar:**
  - Critical handlers (gate_run, gate_report, gate_history): happy path + at least one error path
  - Simple handlers (spec_get, spec_list): single happy-path test minimum
- **Snapshot testing:** Use `insta` for MCP response payloads — consistent with existing schema snapshot pattern, catches serialization regressions.

### 2. Open Issue Triage Scope

**Decision:** Broad sweep of test-related issues across all phases (3-18), strictly limited to test scope.

- **Scope:** Address test-related issues from ALL phases (3-18), not just phases 3 and 6. TEST-02 covers the explicitly called-out phases; TEST-03 covers the rest under "comprehensive tests for new features."
- **Non-test issues excluded:** Refactoring suggestions, doc comments, naming improvements, and code cleanup issues are OUT OF SCOPE. Phase 19 is tests + cargo-deny + dogfooding only.
- **Issue closure:** Each issue individually reviewed and closed with a note explaining resolution. No bulk auto-closing.
- **Staleness audit:** Before writing tests, audit open issues to identify which are still valid after phases 11-18 refactors. Skip issues invalidated by later work. Close stale issues with explanation.

### 3. Dogfooding Spec Design

**Decision:** Comprehensive `self-check.toml` with deterministic AND agent gate criteria.

- **Deterministic criteria (required enforcement):**
  - `cargo fmt --check` (formatting)
  - `cargo clippy` with `-D warnings` (linting)
  - `cargo test` (tests pass)
  - `cargo deny check` (licenses, advisories, sources)
  - Additional checks beyond `just ready`: schema snapshot freshness, or similar project-specific validations
- **Agent gate criteria (advisory enforcement):**
  - At least one `AgentReport` criterion to exercise the full dual-track feature set
  - Advisory enforcement per the trust asymmetry decision from Phase 16
- **Working directory:** Assumes execution from repo root. No explicit path overrides in the spec.
- **Location:** `.assay/specs/self-check.toml`

### 4. Test Organization & Naming

**Decision:** Convention established for new tests; existing tests left as-is.

- **MCP test location:**
  - Direct handler tests: `#[cfg(test)] mod tests` inside `crates/assay-mcp/src/server.rs`
  - Protocol-level tests: `crates/assay-mcp/tests/` directory
- **Naming convention:** `{feature}_{scenario}_{expected}` pattern. No `test_` prefix (Rust's `#[test]` attribute is sufficient). Examples:
  - `spec_get_valid_spec_returns_content`
  - `gate_run_missing_working_dir_returns_error`
  - `gate_report_timeout_finalizes_session`
- **Naming scope:** Apply convention to NEW tests only. Do not rename existing tests (that's refactoring, out of scope).
- **Shared utilities:** Keep helpers local to each test file. No shared test utility crate. Extract only if duplication becomes painful during execution.

## Deferred Ideas

None captured during discussion.

## Scope Boundaries

- IN SCOPE: Test coverage, cargo-deny policy tightening, dogfooding spec
- OUT OF SCOPE: Refactoring, doc comments, naming improvements, code cleanup (even if open issues exist for them)
- FIXED: Phase boundary from ROADMAP.md — no new capabilities added
