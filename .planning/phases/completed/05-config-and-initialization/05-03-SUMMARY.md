# Phase 5 Plan 03: Init Logic and CLI Subcommand Summary

**One-liner:** Implemented `assay init` command with core init logic (template rendering, idempotency guard, project name inference) and CLI subcommand as thin wrapper

## Frontmatter

- **Phase:** 05-config-and-initialization
- **Plan:** 03
- **Subsystem:** init, cli
- **Tags:** init, cli, templates, config-generation, idempotency
- **Completed:** 2026-03-02
- **Duration:** ~15 minutes

### Dependency Graph

- **Requires:** Plan 05-01 (AssayError::AlreadyInitialized variant, AssayError::Io structured fields)
- **Provides:** `assay_core::init` module (init, InitOptions, InitResult), CLI `Init` subcommand
- **Affects:** Phase 6+ (init creates the project structure that specs, gates, etc. operate within)

### Tech Stack

- **Added:** No new dependencies
- **Patterns:** String templates for TOML generation (comments not supported by `toml::to_string`); `create_dir()` for atomic idempotency guard; thin CLI wrapper delegating to core

### Key Files

**Created:**
- `crates/assay-core/src/init.rs` — InitOptions, InitResult, init(), template rendering, 7 tests

**Modified:**
- `crates/assay-core/src/lib.rs` — Added `pub mod init;`
- `crates/assay-cli/src/main.rs` — Init subcommand with --name flag, cargo-style output

### Decisions

| Decision | Rationale |
|----------|-----------|
| String templates for all generated files | `toml::to_string()` cannot emit comments; CONTEXT.md requires commented templates for self-documentation |
| `create_dir()` not `create_dir_all()` for `.assay/` | `create_dir_all` silently succeeds on existing dirs; `create_dir` returns `AlreadyExists` for atomic idempotency check |
| Minimal project name sanitization | Only falls back to "assay-project" when `file_name()` returns None/empty; users can override with `--name` |
| Three created_files in InitResult (config, spec, gitignore) | Directories not tracked in created_files since they're structural, not content artifacts |

### Metrics

- **Tasks:** 2/2 complete
- **Tests:** 7 new init tests, 27 total assay-core tests passing
- **Manual verification:** init, re-init error, --name override all confirmed

## Task Summary

| Task | Name | Commit | Key Changes |
|------|------|--------|-------------|
| 1 | Implement init logic in assay-core | 9ce4919 | init module with InitOptions/InitResult, init(), template rendering (config, spec, gitignore), infer_project_name, 7 tests |
| 2 | Wire Init subcommand in assay-cli | 911ab3e | Init variant in Command enum, --name flag, cargo-style output, error handling with exit code 1 |

## Deviations from Plan

None -- plan executed as written. The config module's `validate()` function was already implemented by the concurrent Plan 02 execution, so no blocking issue arose.

## Verification Results

1. `cargo test -p assay-core -- init` -- all 7 init tests pass
2. `cargo run -p assay-cli -- init --help` -- shows Init subcommand with --name flag
3. Manual init in /tmp/assay-test-init -- creates .assay/config.toml, .assay/specs/hello-world.toml, .assay/.gitignore
4. Manual re-init -- fails with "Error: .assay/ already exists. Remove it first to reinitialize."
5. Manual init with --name custom-name -- config contains `project_name = "custom-name"`
6. `just ready` -- all checks pass (fmt, lint, test, deny)

## Artifacts Produced

### config.toml template
- `project_name` inferred from directory or `--name` override
- `specs_dir = "specs/"` with comment
- Commented-out `[gates]` section with `default_timeout = 300` and `working_dir = "."`
- Documentation URL reference

### hello-world.toml example spec
- Two criteria: `project-builds` (with `cmd = "echo 'hello from assay'"`) and `readme-exists` (no cmd)
- TOML comments explaining every field

### .gitignore
- Ignores `results/` and `*.cache`

## Next Phase Readiness

Phase 5 is complete (all 3 plans done). Phase 6 (Spec Files) is unblocked. The init command creates the project structure (`.assay/`, config, specs dir) that spec loading will operate within.
