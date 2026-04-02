---
estimated_steps: 4
estimated_files: 1
---

# T01: Configure tracing subscriber format and default filter

**Slice:** S01 — M011 Leftover Cleanup — Tracing Migration & Flaky Test Fix
**Milestone:** M012

## Description

Restructure the `tracing_subscriber` initialization in `main.rs` so that:
1. **Default path** (no `SMELT_LOG`/`RUST_LOG` set): bare-message format (no timestamp, no target, no level prefix) with a target-scoped default filter `"smelt_cli=info,smelt_core=info,warn"`. This produces output identical to current `eprintln!` behavior.
2. **Explicit env path** (`SMELT_LOG` or `RUST_LOG` set): full format (timestamp, target, level) with the user-provided filter. This gives operators structured diagnostic output.
3. **TUI file appender path**: always uses full format (levels are useful in log files), with whatever filter is configured.

The key insight: detect whether the env var was explicitly set (not just default). If `EnvFilter::try_from_env("SMELT_LOG")` or `try_from_env("RUST_LOG")` succeeds, the user has set a filter — use full format. If both fail, use bare format with the hardcoded default.

## Steps

1. Read `crates/smelt-cli/src/main.rs` lines 38-62 (subscriber init block) to understand current structure.
2. Refactor the init block: detect explicit env var presence via `std::env::var("SMELT_LOG").is_ok() || std::env::var("RUST_LOG").is_ok()`. Use this boolean to branch format config.
3. For the non-TUI branch: if env var is explicitly set, build subscriber with full format + user filter; if not set, build subscriber with `.without_time().with_target(false).with_level(false)` + default filter `"smelt_cli=info,smelt_core=info,warn"`.
4. For the TUI branch: keep file appender, always use full format, use env filter if set or default filter if not. Verify `cargo test --workspace` and `cargo clippy --workspace -- -D warnings` pass.

## Must-Haves

- [ ] Default subscriber (no env var) uses bare-message format: `.without_time().with_target(false).with_level(false)`
- [ ] Default filter is `"smelt_cli=info,smelt_core=info,warn"` (not `"warn"`)
- [ ] When `SMELT_LOG` or `RUST_LOG` is set, subscriber uses full format with user-provided filter
- [ ] TUI file appender uses full format regardless of env var presence
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` clean

## Verification

- `cargo test --workspace` — all tests pass (no subscriber-related failures)
- `cargo clippy --workspace -- -D warnings` — 0 warnings
- Read `main.rs` and confirm: bare format path uses `.without_time().with_target(false).with_level(false)`; default filter string is `"smelt_cli=info,smelt_core=info,warn"`

## Observability Impact

- Signals added/changed: Default filter changes from `"warn"` to `"smelt_cli=info,smelt_core=info,warn"` — `info!` events from smelt crates now visible by default
- How a future agent inspects this: Read `main.rs` subscriber init block; check `SMELT_LOG` env var for explicit filter override
- Failure state exposed: If filter string is invalid, `EnvFilter::new()` would panic — but hardcoded strings are validated at dev time

## Inputs

- `crates/smelt-cli/src/main.rs` — current subscriber init (lines 38-62)
- S01-RESEARCH.md — format config and filter recommendations
- D107 — subscriber branching pattern (single init before command dispatch)

## Expected Output

- `crates/smelt-cli/src/main.rs` — refactored subscriber init with bare/full format branching and new default filter
