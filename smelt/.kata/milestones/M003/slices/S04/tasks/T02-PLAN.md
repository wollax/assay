---
estimated_steps: 5
estimated_files: 4
---

# T02: Add `smelt init` command

**Slice:** S04 — Infrastructure Hardening
**Milestone:** M003

## Description

`smelt init` removes the blank-page problem for new users: it writes a `./job-manifest.toml` skeleton with all required fields pre-filled with placeholder values and inline comments, ready to edit and run. The generated file must pass `JobManifest::validate()` so users get immediate `--dry-run` feedback without having to hunt for field names.

The skeleton must be written as a raw string literal with embedded `#`-prefixed TOML comments — `toml::to_string_pretty()` strips comments, so struct serialization is not an option (constraint from S04 research and common pitfall section).

## Steps

1. **Create `crates/smelt-cli/src/commands/init.rs`.** Define `#[derive(Debug, Args)] pub struct InitArgs {}` (no fields — the output path is always `./job-manifest.toml`). Define `pub async fn execute(_args: &InitArgs) -> anyhow::Result<i32>`.

2. **Implement the idempotency guard.** In `execute()`, check `std::path::Path::new("job-manifest.toml").exists()`. If true, print to stderr: `Error: job-manifest.toml already exists. Remove it first or edit it directly.` and return `Ok(1)`.

3. **Write the skeleton.** Define a `const SKELETON: &str = r#"..."#;` containing a valid TOML manifest with all required fields filled with realistic placeholder strings and inline comments. Required fields that must be non-empty for `validate()` to pass:
   - `job.name` — e.g. `"my-job"`
   - `job.repo` — e.g. `"."` (works for `--dry-run`; users edit to an absolute path for real runs)
   - `job.base_ref` — e.g. `"main"`
   - `environment.runtime` — must be `"docker"`
   - `environment.image` — e.g. `"ubuntu:22.04"`
   - `credentials.provider` — e.g. `"anthropic"`
   - `credentials.model` — e.g. `"claude-sonnet-4-20250514"`
   - At least one `[[session]]` with non-empty `name`, `spec`, `harness`, and `timeout > 0`
   - `merge.strategy` — e.g. `"sequential"`
   - `merge.target` — e.g. `"main"`
   Call `std::fs::write("job-manifest.toml", SKELETON)?;` and print to stdout: `Created job-manifest.toml — edit and run with: smelt run job-manifest.toml`. Return `Ok(0)`.

4. **Register in `mod.rs` and `main.rs`.** Add `pub mod init;` to `commands/mod.rs`. Add `/// Generate a skeleton job manifest Init(commands::init::InitArgs),` to the `Commands` enum in `main.rs`. Add handler `Commands::Init(ref args) => commands::init::execute(args).await,` to the match arm.

5. **Add unit tests in `init.rs`.** Three tests, each using `TempDir` and `std::env::set_current_dir`:
   - `test_init_creates_manifest`: change into a TempDir, call `execute()`, assert `Ok(0)`, assert `job-manifest.toml` exists, load with `JobManifest::load()`, validate with `manifest.validate()`, assert `validate()` returns `Ok(())`.
   - `test_init_fails_if_file_exists`: create `job-manifest.toml` manually, call `execute()`, assert `Ok(1)`.
   - `test_init_skeleton_parses`: parse the `SKELETON` const directly with `toml::from_str::<smelt_core::manifest::JobManifest>(SKELETON)`, assert `Ok(_)` — quick compile-time check that the skeleton is valid TOML.

## Must-Haves

- [ ] `smelt init` exits 0 and creates `./job-manifest.toml` when the file does not exist
- [ ] The created file passes `JobManifest::validate()` without modification
- [ ] `smelt init` exits 1 with a clear error message if `job-manifest.toml` already exists
- [ ] The skeleton contains inline `#`-comments explaining each section
- [ ] `smelt --help` shows `init` as a subcommand
- [ ] `cargo test -p smelt-cli` — 3 new init tests pass

## Verification

- `cargo test -p smelt-cli -p smelt-core` passes with all new tests
- `test_init_creates_manifest` asserts: `JobManifest::load("job-manifest.toml").is_ok()` and `manifest.validate().is_ok()`
- `test_init_fails_if_file_exists` asserts: return code is 1
- `test_init_skeleton_parses` asserts: `toml::from_str::<JobManifest>(SKELETON).is_ok()`

## Observability Impact

- Signals added/changed: `smelt init` prints the exact command to run next (`smelt run job-manifest.toml`) — minimal but useful onboarding signal
- How a future agent inspects this: `smelt run job-manifest.toml --dry-run` will validate the generated file; a failure there signals a regression in the skeleton's validity
- Failure state exposed: if the idempotency guard is missing, a second `smelt init` would silently overwrite user edits — the guard and its test make this explicit

## Inputs

- T01 must be complete (monitor/state path is stable before writing new CLI commands)
- `crates/smelt-core/src/manifest.rs` — `JobManifest::load()`, `JobManifest::validate()` — needed to verify skeleton in tests
- `examples/job-manifest.toml` — reference for field names and realistic placeholder values (D065: skeleton is raw string literal, not struct serialization)

## Expected Output

- `crates/smelt-cli/src/commands/init.rs` — new file: `InitArgs`, `execute()`, `SKELETON` const, 3 unit tests
- `crates/smelt-cli/src/commands/mod.rs` — `pub mod init;` added
- `crates/smelt-cli/src/main.rs` — `Init` variant added to `Commands`; match arm added
