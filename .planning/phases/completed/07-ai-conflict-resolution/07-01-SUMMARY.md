# Phase 07 Plan 01: AI Provider Abstraction Layer Summary

**One-liner:** AiProvider trait with RPITIT, GenAiProvider backed by genai crate, 3-way prompt templates, AiConfig from TOML

## What Was Done

### Task 1: Workspace deps, error variant, AiConfig, AiProvider trait

- Added `genai = "0.5"` and `similar = { version = "2", features = ["unicode"] }` as workspace dependencies
- Moved `serde_json` from dev-dependencies to dependencies in smelt-core (needed for AI response handling)
- Added `SmeltError::AiResolution { message: String }` variant (19th variant)
- Created `crates/smelt-core/src/ai/mod.rs` with:
  - `AiConfig` struct (enabled, provider, model, max_retries, api_key, endpoint) with serde Deserialize + Serialize
  - `AiConfig::load(smelt_dir)` reads `.smelt/config.toml` `[ai]` section via `ConfigFile` wrapper
  - `AiConfig::default()` with enabled=true, max_retries=2
  - `AiProvider` trait with RPITIT `complete()` method (no async-trait crate)
- Added `pub mod ai;` to `lib.rs` with re-exports: `AiConfig`, `AiProvider`, `GenAiProvider`
- Updated `init.rs` `DEFAULT_CONFIG` with commented-out `[ai]` section

### Task 2: GenAiProvider implementation + prompt template construction

- Created `crates/smelt-core/src/ai/provider.rs` with:
  - `GenAiProvider` wrapping `genai::Client`
  - `GenAiProvider::new(config)` with API key injection via env var for known providers
  - `AiProvider::complete()` builds `ChatRequest` with system + user messages, calls `exec_chat`, maps errors to `SmeltError::AiResolution`
  - `strip_code_fences()` post-processor for LLM responses (handles with/without language tag, extra whitespace, missing closing fence)
  - `provider_to_env_key()` maps provider names to env var names
- Created `crates/smelt-core/src/ai/prompt.rs` with:
  - `build_system_prompt()` — static system prompt instructing raw file output only
  - `build_resolution_prompt()` — 3-way merge context with file path, session name, task desc, commit subjects
  - `build_retry_prompt()` — appends user feedback to original prompt

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Edition 2024 unsafe set_var**

- **Found during:** Task 1 (compilation)
- **Issue:** `std::env::set_var` is unsafe in Rust edition 2024
- **Fix:** Wrapped in `unsafe` block with safety comment
- **Files modified:** `crates/smelt-core/src/ai/provider.rs`

**2. [Rule 3 - Blocking] Tasks 1 and 2 merged**

- **Found during:** Task 1
- **Issue:** `ai/mod.rs` re-exports types from `provider.rs` and `prompt.rs` — these files must exist for Task 1 to compile
- **Fix:** Implemented both modules fully in Task 1. Task 2 had no additional changes to commit.
- **Impact:** Single commit instead of two; all functionality delivered.

## Test Results

- 135 tests pass (all existing + 14 new AI module tests)
- `cargo clippy -p smelt-core -- -D warnings` clean

## Decisions Made

- `set_var` wrapped in unsafe with safety comment — called during single-threaded client construction
- GenAiProvider uses `Client::default()` (genai's built-in model-to-provider mapping) rather than custom `ServiceTargetResolver`
- API key from config injected via env var passthrough (only if env var not already set — env takes precedence)
- `strip_code_fences` is conservative — only strips if both opening and closing fences present

## Files

### Created

- `crates/smelt-core/src/ai/mod.rs` — AiProvider trait, AiConfig, module root
- `crates/smelt-core/src/ai/provider.rs` — GenAiProvider implementation
- `crates/smelt-core/src/ai/prompt.rs` — Prompt template construction

### Modified

- `Cargo.toml` — genai and similar workspace deps
- `Cargo.lock` — dependency resolution
- `crates/smelt-core/Cargo.toml` — genai and serde_json dependencies
- `crates/smelt-core/src/error.rs` — AiResolution variant
- `crates/smelt-core/src/init.rs` — DEFAULT_CONFIG with [ai] section
- `crates/smelt-core/src/lib.rs` — pub mod ai + re-exports

## Commits

- `7621b54`: feat(07-01): add AI provider abstraction, AiConfig, and error variant

## Duration

~5 minutes

## Next Phase Readiness

Plan 02 (AiConflictHandler) can proceed. All foundation types are in place:
- `AiProvider` trait for dependency injection
- `GenAiProvider` for production use
- Prompt builders for 3-way merge context
- `AiConfig` for configuration loading
- `SmeltError::AiResolution` for error propagation
