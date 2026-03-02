# Plan 05-02 Summary: Config Loading and Validation

**Phase:** 05-config-and-initialization
**Plan:** 02
**Type:** TDD (red-green-refactor)
**Status:** Complete
**Duration:** ~16 minutes

## One-Liner

Implemented `from_str()`, `validate()`, and `load()` free functions in `assay-core::config` via TDD with 17 tests covering parsing, validation, and filesystem loading.

## Tasks Completed

| # | Task | Type | Status |
|---|------|------|--------|
| 1 | Write failing tests for `from_str()` | RED | Done |
| 2 | Implement `from_str()` | GREEN | Done |
| 3 | Write failing tests for `validate()` | RED | Done |
| 4 | Implement `validate()` | GREEN | Done |
| 5 | Write failing tests for `load()` | RED | Done |
| 6 | Implement `load()` | GREEN | Done |
| 7 | Run `just ready` | Verification | Done |

## Commits

| Hash | Message |
|------|---------|
| `40978d0` | test(05-02): add failing tests for from_str() |
| `81d5938` | feat(05-02): implement from_str() for config parsing |
| `1c6f092` | test(05-02): add failing tests for validate() |
| `389c156` | feat(05-02): implement validate() for config validation |
| `5835b7d` | test(05-02): add failing tests for load() |
| `7c1089f` | feat(05-02): implement load() for config file loading |
| `601d1f4` | refactor(05-02): clippy let-chain and fmt fixes |

## What Was Built

### `from_str(s: &str) -> Result<Config, toml::de::Error>`
- One-liner wrapping `toml::from_str::<Config>()`
- Returns raw toml error with line/column info (composable API)
- Rejects unknown fields via `deny_unknown_fields` on Config/GatesConfig

### `validate(config: &Config) -> std::result::Result<(), Vec<ConfigError>>`
- Collects **all** validation errors at once (not fail-fast)
- Checks: empty/whitespace project_name, empty specs_dir, zero default_timeout
- Returns `Ok(())` when valid, `Err(Vec<ConfigError>)` otherwise

### `load(root: &Path) -> Result<Config>`
- Reads `root/.assay/config.toml`
- Parses with `from_str()`, wraps parse errors in `AssayError::ConfigParse` (with file path)
- Validates with `validate()`, wraps validation errors in `AssayError::ConfigValidation`
- IO errors wrapped in `AssayError::Io` with operation context

## Test Coverage

- **from_str:** 6 tests (valid all fields, minimal with defaults, gates defaults, invalid syntax, unknown top-level keys, unknown gates keys)
- **validate:** 7 tests (valid config, valid with gates, empty project_name, whitespace project_name, empty specs_dir, zero timeout, multi-error collection)
- **load:** 4 tests (valid config, missing file, invalid TOML, invalid semantics)

## Dependencies Added

- `tempfile = "3"` added to workspace and assay-core dev-dependencies for filesystem test fixtures

## Deviations

1. **Pre-existing fmt issue in init.rs** -- `just fmt` auto-fixed a long line in `init.rs` test helper (from Plan 01). Included in refactor commit.
2. **Clippy collapsible_if** -- Nested `if let Some` / `if` in `validate()` collapsed to let-chain per clippy lint. Applied during refactor phase.

## Decisions Made

- `from_str()` returns `toml::de::Error` (not `AssayError`) for composability -- tests/tools can inspect raw parse details
- `validate()` returns `Vec<ConfigError>` (not `AssayError`) -- separates error collection from error reporting
- `load()` composes both and wraps into `AssayError` variants with file path context
- Used `tempfile` crate for filesystem test isolation (temporary directories)
