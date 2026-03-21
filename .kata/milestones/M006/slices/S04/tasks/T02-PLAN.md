---
estimated_steps: 4
estimated_files: 2
---

# T02: Add `config_save` to `assay-core::config` and test atomic write

**Slice:** S04 — Provider Configuration Screen
**Milestone:** M006

## Description

Add `pub fn config_save(root: &Path, config: &Config) -> Result<()>` to `assay-core::config` using the established NamedTempFile + sync_all + persist atomic-write pattern (same as `milestone_save`). The TUI must not write config files directly — all persistence goes through `assay-core` to maintain the clean dependency graph (D093). Extend the existing `config_provider.rs` test file from T01 with three `config_save` tests.

## Steps

1. Open `crates/assay-core/src/config/mod.rs`. Add the `config_save` function after the existing `load` function:
   ```rust
   /// Atomically write `config` to `<root>/.assay/config.toml`.
   ///
   /// Uses a NamedTempFile in the same directory as the target file to
   /// ensure atomic rename semantics. A crash mid-write never leaves a
   /// corrupt `config.toml`.
   pub fn config_save(root: &Path, config: &Config) -> Result<()> {
       let assay_dir = root.join(".assay");
       let final_path = assay_dir.join("config.toml");

       let content = toml::to_string_pretty(config).map_err(|e| AssayError::Io {
           operation: "serializing config".into(),
           path: final_path.clone(),
           source: std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()),
       })?;

       let mut tmpfile = NamedTempFile::new_in(&assay_dir)
           .map_err(|e| AssayError::io("creating temp file for config", &assay_dir, e))?;
       tmpfile
           .write_all(content.as_bytes())
           .map_err(|e| AssayError::io("writing config", &final_path, e))?;
       tmpfile
           .as_file()
           .sync_all()
           .map_err(|e| AssayError::io("fsyncing config", &final_path, e))?;
       tmpfile
           .persist(&final_path)
           .map_err(|e| AssayError::io("persisting config", &final_path, e.error))?;

       Ok(())
   }
   ```
   Ensure `use std::io::Write;` is present in the imports (check if already imported for other functions).

2. Verify that `config_save` is accessible via `assay_core::config::config_save` — it must be declared `pub` in the module. Check whether `mod.rs` re-exports via `pub use` or if callers use the full path `assay_core::config::config_save`. For `assay-tui`, the full path is fine.

3. Extend `crates/assay-core/tests/config_provider.rs` with three `config_save` tests:
   - `config_save_creates_file`: Create a tempdir with `.assay/` subdir, call `config_save(dir.path(), &config)` — assert the file exists and `config::load(dir.path())` returns the same config.
   - `config_save_overwrites_existing`: Write an initial config, call `config_save` with a modified config — assert `config::load` returns the updated values.
   - `config_save_with_provider_persists`: Build a `Config` with `provider: Some(ProviderConfig { provider: ProviderKind::OpenAI, planning_model: Some("gpt-4o-mini".into()), ..Default::default() })`, call `config_save`, then `config::load` — assert `provider.unwrap().provider == ProviderKind::OpenAI` and `provider.unwrap().planning_model == Some("gpt-4o-mini")`.

4. Run `cargo test -p assay-core config_provider` — all five tests (two from T01 + three from T02) must pass.

## Must-Haves

- [ ] `config_save` is `pub` and exported from `assay-core::config`
- [ ] Uses NamedTempFile::new_in (same directory as target) for atomic rename
- [ ] Calls `sync_all()` before `persist()` — no skipping the fsync
- [ ] All `AssayError::Io` / `AssayError::io` calls carry the correct path and operation label
- [ ] `config_save_with_provider_persists` test proves the full round-trip including provider field

## Verification

- `cargo test -p assay-core config_provider` — all 5 tests pass (2 from T01 + 3 from T02)
- `cargo test -p assay-core` — no regressions

## Observability Impact

- Signals added/changed: `AssayError::Io { operation, path, source }` on any failure — operation label identifies which step failed (creating temp file, writing, fsyncing, persisting)
- How a future agent inspects this: check `.assay/config.toml` exists and is valid TOML after a save; `config::load(root)` to read back
- Failure state exposed: `persist` errors include the original error from `tempfile::PathPersistError`; error path is always the final `config.toml` path, not the temp path

## Inputs

- `crates/assay-core/src/config/mod.rs` — existing `load` function (pattern to follow for error handling)
- `crates/assay-core/src/milestone/mod.rs` — `milestone_save` (the exact pattern to replicate for atomic write)
- `crates/assay-core/tests/config_provider.rs` — T01's roundtrip tests (extend, don't replace)
- T01 complete — `ProviderKind`, `ProviderConfig`, `Config.provider` field must exist before these tests can compile

## Expected Output

- `crates/assay-core/src/config/mod.rs` — `config_save` function added; exported as `pub`
- `crates/assay-core/tests/config_provider.rs` — three new `config_save` tests added (total 5 in file)
