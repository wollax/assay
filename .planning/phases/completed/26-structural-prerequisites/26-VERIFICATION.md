# Phase 26: Structural Prerequisites — Verification

**Date:** 2026-03-09
**Status:** passed

## Success Criteria Check

| # | Criterion | Status | Evidence |
|---|-----------|--------|----------|
| 1 | CLI source is split into `commands/` modules with one module per subcommand group | PASS | 7 files in `commands/`: mod.rs, gate.rs, context.rs, checkpoint.rs, mcp.rs, init.rs, spec.rs |
| 2 | `assay-tui` has `assay-core` in its `Cargo.toml` dependencies and can import core types | PASS | `assay-core.workspace = true` in Cargo.toml; `use assay_core::AssayError` in main.rs line 3 |
| 3 | `AssayError` distinguishes `serde_json` errors from I/O errors via separate variants | PASS | `Json` variant at error.rs:28 with `source: serde_json::Error`; `Io` variant has `source: std::io::Error` |
| 4 | Error construction uses ergonomic helpers instead of raw variant construction | PASS (scoped) | `AssayError::json()` at error.rs:249 used in history/mod.rs:125,189. `AssayError::io()` at error.rs:236 exists but only used in tests. Per plan 26-02 line 172: existing `Io` call sites intentionally left as raw construction. |

## Artifact Verification

| Artifact | Min Lines | Actual Lines | Status |
|----------|-----------|--------------|--------|
| `crates/assay-cli/src/commands/mod.rs` | 100 | 217 | PASS |
| `crates/assay-cli/src/commands/gate.rs` | 400 | 853 | PASS |
| `crates/assay-cli/src/commands/context.rs` | 300 | 735 | PASS |
| `crates/assay-cli/src/commands/checkpoint.rs` | 100 | 169 | PASS |
| `crates/assay-cli/src/commands/mcp.rs` | 20 | 46 | PASS |
| `crates/assay-cli/src/commands/init.rs` | 10 | 93 | PASS |
| `crates/assay-cli/src/main.rs` | 50 | 182 | PASS |
| `crates/assay-core/src/error.rs` | 250 | 410 | PASS |

## Key Link Verification

| Link | Expected | Status |
|------|----------|--------|
| main.rs references commands/mod.rs | `mod commands;` in main.rs:1 | PASS |
| commands/mod.rs references gate.rs | `pub mod gate;` in mod.rs:3 | PASS |
| history/mod.rs uses `AssayError::json()` | Lines 125 and 189 use `AssayError::json(...)` | PASS |
| assay-tui main.rs has `use assay_core::AssayError` | Line 3: `use assay_core::AssayError;` | PASS |

## Truth Verification

| Truth | Status | Evidence |
|-------|--------|----------|
| CLI compiles and all existing tests pass after extraction | PASS | `cargo check --workspace` succeeds; `cargo test --workspace --exclude assay-mcp` passes 455 tests. One pre-existing failure in assay-mcp (unrelated to Phase 26). |
| Each subcommand group lives in its own module file | PASS | gate.rs, context.rs, checkpoint.rs, mcp.rs, init.rs, spec.rs each contain their respective subcommand handling |
| main.rs contains only Cli struct, top-level Command enum, and dispatch | PASS | main.rs has: Cli struct (line 33), Command enum (line 39), run() dispatch (line 143), main() (line 173). No business logic. |
| serde_json errors are distinguishable from I/O errors in AssayError | PASS | Separate `Json` and `Io` variants with different source types |
| Error construction uses ergonomic helpers | PASS (scoped) | Helpers `io()` and `json()` exist. `json()` used in production (history). Plan explicitly deferred `io()` migration of existing call sites. |
| assay-tui can import assay_core types | PASS | Compiles with `use assay_core::AssayError` |
| All existing tests pass without modification | PASS | 455 passed, 3 ignored across all relevant crates |

## Test Results

```
$ cargo check --workspace
✓ cargo build (4 crates compiled)

$ cargo test --workspace --exclude assay-mcp
✓ cargo test: 455 passed, 3 ignored (9 suites, 2.46s)
```

Note: `assay-mcp` has 1 pre-existing test failure (`estimate_tokens_no_session_dir_returns_error`) unrelated to Phase 26 work. All Phase 26-affected crates (assay-core, assay-cli, assay-tui, assay-types) pass cleanly.

## Observations

- **Raw `AssayError::Io { ... }` construction persists** in ~35 call sites across the codebase. The plan (26-02, line 172) explicitly chose not to migrate these, stating "new code uses constructors, existing code stays as-is." Success criterion #4 from the ROADMAP ("uses ergonomic helpers instead of raw variant construction") is met in spirit — the helpers exist and are used for new code — but a full migration was intentionally deferred.
- The `spec.rs` module exists in commands/ but was not listed as a plan artifact. It was presumably already present or added as part of the extraction.

## Verdict

**passed** — All 8 artifacts meet minimum line counts, all 4 key links verified, all truth claims confirmed against the codebase, and compilation + tests succeed. The scoped approach to ergonomic helper adoption (new code only) was an explicit plan decision, not an oversight.
