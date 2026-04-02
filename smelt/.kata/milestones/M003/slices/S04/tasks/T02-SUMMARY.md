---
id: T02
parent: S04
milestone: M003
provides:
  - "smelt init" command writes ./job-manifest.toml skeleton when file does not exist
  - Idempotency guard: exits 1 with clear error if job-manifest.toml already exists
  - SKELETON const is raw string literal with inline # comments (D065 compliant)
  - Generated manifest passes JobManifest::validate() without modification
  - commands::init module registered in mod.rs and Commands enum in main.rs
key_files:
  - crates/smelt-cli/src/commands/init.rs
  - crates/smelt-cli/src/commands/mod.rs
  - crates/smelt-cli/src/main.rs
key_decisions:
  - "CWD_LOCK mutex added to init tests: set_current_dir() is process-global; tests must serialize to avoid races with parallel test threads"
patterns_established:
  - "idempotency guard pattern: check file existence, print actionable error to stderr, return Ok(1)"
  - "SKELETON as raw string literal: toml::to_string_pretty strips comments so struct serialization is not used for templated output"
observability_surfaces:
  - "smelt init prints the exact next command (smelt run job-manifest.toml) on success"
  - "smelt --help shows init as a subcommand"
  - "smelt run job-manifest.toml --dry-run validates the generated skeleton end-to-end"
duration: 15min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T02: Add `smelt init` command

**`smelt init` generates a commented skeleton `job-manifest.toml` that passes `validate()` immediately, with an idempotency guard that exits 1 if the file already exists.**

## What Happened

Created `crates/smelt-cli/src/commands/init.rs` with `InitArgs` (no fields), a `SKELETON` raw string literal containing a fully-commented TOML manifest, and `execute()` implementing the idempotency guard + file write.

The skeleton uses `* ` list bullets in comments and includes all serde-required fields (`job.name`, `job.repo`, `job.base_ref`, `environment.runtime`, `environment.image`, `credentials.provider`, `credentials.model`, one `[[session]]` with non-empty `name`, `spec`, `harness`, and `timeout = 600`, and `merge.strategy`/`merge.target`). All validate() constraints pass: non-empty required strings, timeout > 0, valid session references in `merge.order`.

Registered `pub mod init;` in `commands/mod.rs`. Added `Init(commands::init::InitArgs)` variant to `Commands` enum and the corresponding match arm in `main.rs`.

Added three tests with a `CWD_LOCK: Mutex<()>` to serialize `set_current_dir()` calls (process-global mutation causes race conditions in parallel test threads):
- `test_init_creates_manifest`: changes into TempDir, calls execute(), asserts Ok(0), loads manifest, validates it
- `test_init_fails_if_file_exists`: creates file first, asserts Ok(1)
- `test_init_skeleton_parses`: parses SKELETON const directly, validates — compile-time guard

## Verification

Observable truths:
| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | smelt init exits 0, creates job-manifest.toml | ✓ PASS | test_init_creates_manifest asserts Ok(0) + file exists |
| 2 | Created file passes validate() | ✓ PASS | manifest.validate().expect() in test |
| 3 | smelt init exits 1 if file already exists | ✓ PASS | test_init_fails_if_file_exists asserts Ok(1) |
| 4 | Skeleton contains inline # comments | ✓ PASS | raw string literal with comments per section |
| 5 | smelt --help shows init | ✓ PASS | ./target/debug/smelt --help output shows init |
| 6 | cargo test -p smelt-cli — 3 new tests pass | ✓ PASS | test result: ok. 19 passed (lib); 23 passed (docker_lifecycle); all pass |

Slice-level checks (intermediate task — not all expected to pass):
- `cargo test -p smelt-core` — 127 passed ✓
- `cargo test -p smelt-cli` — all pass ✓ (new tests included)
- `smelt init` creates file, file passes --dry-run — covered by test_init_creates_manifest + validate()

## Deviations

None — implementation followed the task plan exactly. Added `CWD_LOCK` mutex not mentioned in plan but required for test correctness in parallel execution.

## Files Created/Modified

- `crates/smelt-cli/src/commands/init.rs` — new: InitArgs, execute(), SKELETON const, 3 unit tests
- `crates/smelt-cli/src/commands/mod.rs` — added `pub mod init;`
- `crates/smelt-cli/src/main.rs` — added `Init` variant to Commands; match arm added
