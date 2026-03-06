# Plan 20-02 Summary: Session Diagnostics Engine

## Outcome

All 3 tasks completed. The core session parsing, discovery, token extraction, and diagnostics engine is implemented in `assay-core::context`.

## What was built

### Module structure (`crates/assay-core/src/context/`)

- **mod.rs** (85 lines) - Public API: `list_sessions()`, `quick_line_count()`, re-exports from submodules
- **parser.rs** (105 lines) - Line-by-line JSONL parser with `ParsedEntry` wrapper and per-line error tolerance
- **discovery.rs** (130 lines) - Session file discovery via project slug, `find_session_dir()`, `resolve_session()`, `sessions_from_history()`
- **tokens.rs** (155 lines) - Token extraction (exact from usage data + heuristic from bytes), `quick_token_estimate()` tail-read, `estimate_tokens()` with health indicator, `is_sidechain()` filter
- **diagnostics.rs** (210 lines) - `diagnose()` producing full `DiagnosticsReport`, `categorize_bloat()` with all 6 categories

### Supporting changes

- **error.rs** - 3 new error variants: `SessionDirNotFound`, `SessionFileNotFound`, `SessionParse`
- **Cargo.toml** - Added `dirs` and `regex-lite` dependencies
- **deny.toml** - Added `MPL-2.0` license allowance (for `option-ext` via `dirs-sys`) and `getrandom@0.2` skip (for `redox_users` via `dirs-sys`)

## Tests

24 new unit tests across all 4 submodules:

- **parser** (4): valid entries, empty/malformed lines, unknown types, file not found
- **discovery** (5): slug conversion (3 cases), discover filtering, resolve by ID, resolve latest, resolve missing
- **tokens** (4): extract usage with sidechain filtering, sidechain-only returns None, byte estimation, context window lookup
- **diagnostics** (7): complete report, progress ticks, thinking blocks, metadata entries, system reminders, tool output, stale reads, all 6 categories always present

## Key decisions

- `sessions_from_history()` and `estimate_tokens_from_bytes()` marked `#[allow(dead_code)]` -- public API consumed by CLI/MCP in later plans
- Constants `DEFAULT_CONTEXT_WINDOW` and `SYSTEM_OVERHEAD_TOKENS` are `pub(super)` for use within the context module
- `is_sidechain()` is `pub(super)` for reuse in diagnostics
- Stale read detection tracks `(file_path)` in a HashSet; second read of same path counts as stale
- Used Rust 2024 edition `let` chains for collapsible conditionals (clippy-compliant)

## Commits

1. `48a35a1` feat(20-02): add context module skeleton with parser, discovery, and tokens stubs
2. `636caf2` feat(20-02): implement bloat categorization with stale read detection and tests
3. `b2835c8` fix(20-02): resolve clippy lints, formatting, and cargo-deny violations

## Verification

- `just ready` passes (fmt-check, lint, test, deny)
- 371 total tests pass across workspace (24 new context tests)
