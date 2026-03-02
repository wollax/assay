---
phase: 05-config-and-initialization
status: passed
score: 23/23
verified_by: phase-verifier
date: 2026-03-01
---

# Phase 5 Verification Report

**Phase goal:** Users can initialize an Assay project and the system can load/validate its configuration.

**Test suite:** 54 tests, 0 failures across all workspace crates.

**Schema freshness:** `cargo run --example generate-schemas` produced no diff against committed schemas. All 9 schema files are up to date.

---

## Must-Have Results

### Plan 01 Must-Haves (Config Type Design)

**1. Config type has `project_name`, `specs_dir`, and optional `gates` fields (not `workflows`)**
- PASSED. `assay-types/src/lib.rs` lines 74-86 defines `Config` with exactly these fields. No `workflows` field exists.

**2. `GatesConfig` type exists with `default_timeout` (u64, default 300) and optional `working_dir`**
- PASSED. `assay-types/src/lib.rs` lines 96-106:
  ```rust
  pub struct GatesConfig {
      #[serde(default = "default_timeout")]
      pub default_timeout: u64,
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub working_dir: Option<String>,
  }
  fn default_timeout() -> u64 { 300 }
  ```

**3. Both `Config` and `GatesConfig` reject unknown TOML keys via `deny_unknown_fields`**
- PASSED. `assay-types/src/lib.rs`:
  - `Config` line 73: `#[serde(deny_unknown_fields)]`
  - `GatesConfig` line 97: `#[serde(deny_unknown_fields)]`
  - Covered by tests `from_str_rejects_unknown_keys` and `from_str_rejects_unknown_gates_keys` in `config/mod.rs`.

**4. `AssayError` has `ConfigParse`, `ConfigValidation`, and `AlreadyInitialized` variants**
- PASSED. `assay-core/src/error.rs`:
  - `ConfigParse` (line 27): holds `path: PathBuf` and `message: String`
  - `ConfigValidation` (line 35): holds `path: PathBuf` and `errors: Vec<ConfigError>`
  - `AlreadyInitialized` (line 45): unit variant with display ".assay/ already exists. Remove it first to reinitialize."

**5. All existing tests pass with the redesigned Config type**
- PASSED. `cargo test --workspace` reports 54 passed, 0 failed.

**6. Generated `config.schema.json` reflects the new Config shape**
- PASSED. `schemas/config.schema.json` contains `project_name` (required), `specs_dir` (default "specs/"), `gates` (optional `$ref` to `GatesConfig`), and `"additionalProperties": false`. Schema includes `GatesConfig` inline in `$defs` with `default_timeout` (uint64, default 300) and `working_dir` (nullable string).

---

### Plan 02 Must-Haves (Config Loading)

**7. `from_str()` parses valid TOML into a `Config` struct without validation**
- PASSED. `assay-core/src/config/mod.rs` line 32-34:
  ```rust
  pub fn from_str(s: &str) -> std::result::Result<Config, toml::de::Error> {
      toml::from_str(s)
  }
  ```
  Tests `from_str_valid_all_fields` and `from_str_minimal_uses_defaults` verify this.

**8. `from_str()` returns `toml::de::Error` on invalid TOML (with line/column info)**
- PASSED. `from_str_invalid_toml_syntax` test verifies the error contains `"TOML parse error"` (which the toml crate includes alongside position information).

**9. `from_str()` rejects unknown TOML keys via `deny_unknown_fields`**
- PASSED. Tests `from_str_rejects_unknown_keys` and `from_str_rejects_unknown_gates_keys` verify the error message contains `"unknown field"` and the offending key name.

**10. `load()` reads `.assay/config.toml`, parses it, validates it, and returns `Config`**
- PASSED. `assay-core/src/config/mod.rs` lines 79-98. `load_valid_config` test confirms end-to-end.

**11. `load()` returns `AssayError::ConfigParse` with file path when TOML is invalid**
- PASSED. `load_invalid_toml_returns_config_parse` test asserts `path.ends_with("config.toml")` and `message.contains("TOML parse error")`.

**12. `load()` returns `AssayError::ConfigValidation` with all errors when config is semantically invalid**
- PASSED. `load_valid_toml_invalid_semantics_returns_config_validation` test asserts the variant, path, and non-empty errors.

**13. `validate()` collects all validation errors and reports them at once**
- PASSED. `validate_collects_all_errors_at_once` test constructs a config with 3 invalid fields and asserts `errors.len() == 3`.

**14. `validate()` rejects empty `project_name`, whitespace-only `project_name`, empty `specs_dir`, zero `default_timeout`**
- PASSED. Covered by four dedicated tests:
  - `validate_empty_project_name`
  - `validate_whitespace_only_project_name`
  - `validate_empty_specs_dir`
  - `validate_zero_default_timeout`

---

### Plan 03 Must-Haves (Init Command)

**15. `assay init` in a fresh directory creates `.assay/config.toml`, `.assay/specs/`, `.assay/.gitignore`, and `.assay/specs/hello-world.toml`**
- PASSED. `assay-core/src/init.rs` `test_init_creates_all_artifacts` asserts all four paths exist. `InitResult.created_files` reports 3 files (`.assay/config.toml`, `.assay/specs/hello-world.toml`, `.assay/.gitignore`).

**16. `assay init` a second time fails with a clear error mentioning `.assay/` already exists**
- PASSED. `test_init_refuses_existing` asserts `AssayError::AlreadyInitialized`. The display message is: `.assay/ already exists. Remove it first to reinitialize.`

**17. Config template includes commented-out `[gates]` section and `project_name` inferred from directory**
- PASSED. `test_init_config_template_has_comments` asserts the generated config.toml contains `# [gates]` and `# default_timeout`. `test_init_infers_project_name` asserts the inferred name appears as `project_name = "my-cool-project"`.

**18. Example spec has both a runnable criterion (with `cmd`) and a descriptive-only criterion (without `cmd`)**
- PASSED. `test_init_example_spec_has_both_criteria_modes` asserts presence of `cmd =` and `# No \`cmd\`` in `hello-world.toml`.

**19. `assay init --name custom-name` uses the provided name instead of inferring from directory**
- PASSED. `test_init_name_override` asserts `result.project_name == "custom"` and config.toml contains `project_name = "custom"`. CLI wires `--name` flag to `InitOptions { name }` in `assay-cli/src/main.rs` line 64.

**20. Init output follows cargo-style format: `Created assay project \`name\``**
- PASSED. `assay-cli/src/main.rs` line 67:
  ```rust
  println!("  Created assay project `{}`", result.project_name);
  ```
  Format matches the plan specification exactly (leading two spaces, backtick-quoted name). This is the cargo-style format as specified in `05-03-PLAN.md`.

---

## Additional CFG Requirement Checks

**CFG-01: `assay init` creates `.assay/` directory with `config.toml` and `specs/` subdirectory**
- PASSED. Verified by `test_init_creates_all_artifacts`.

**CFG-02: Template-based `config.toml` generation with project name inferred from directory and sensible defaults**
- PASSED. `render_config_template()` in `init.rs` lines 111-132 generates the template; `infer_project_name()` handles directory inference with "assay-project" fallback.

**CFG-03: Example spec file created in `.assay/specs/` during init**
- PASSED. `specs/hello-world.toml` created via `render_example_spec()` at `init.rs` line 75.

**CFG-04: Idempotent init — refuse to overwrite existing `.assay/` directory**
- PASSED. `init.rs` uses `std::fs::create_dir()` (not `create_dir_all`) and maps `AlreadyExists` to `AssayError::AlreadyInitialized`.

**CFG-05: Config loading via `assay_core::config::load()` and `from_str()` free functions**
- PASSED. Both are public free functions in `assay-core/src/config/mod.rs` exposed via `pub mod config` in `assay-core/src/lib.rs`.

**CFG-06: Config validation via `assay_core::config::validate()` with structured error reporting**
- PASSED. `validate()` returns `Err(Vec<ConfigError>)` where each `ConfigError` has `field` and `message`. `AssayError::ConfigValidation` wraps the `Vec<ConfigError>` with file path.

---

## Summary

All 23 must-haves verified against actual source code. Test suite is green (54/54). Schemas are current. No gaps found.
