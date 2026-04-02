---
estimated_steps: 7
estimated_files: 1
---

# T02: Write Contract Unit Tests and Verify Full Suite

**Slice:** S01 — Fix AssayInvoker — Real Assay Contract
**Milestone:** M002

## Description

Write the complete unit test suite for the new `AssayInvoker` API. Tests are the verification artifact for this slice — they assert every contract invariant from the boundary map: `[[sessions]]` key (not `[[session]]`), spec-name references (not inline descriptions), absence of unknown fields, spec TOML structure, sanitization behaviour, and `--base-branch` presence. After tests pass, run the full workspace test suite to confirm nothing regressed.

## Steps

1. Open `crates/smelt-core/src/assay.rs`. Add `#[cfg(test)] mod tests { use super::*; use std::path::Path; }`.
2. Add the `test_manifest(sessions_toml: &str) -> JobManifest` helper (same pattern as the old test suite: wraps a valid manifest header around the sessions TOML, parses with `JobManifest::from_str`).
3. Write **`test_run_manifest_uses_sessions_key_plural`**: create a single-session manifest, call `build_run_manifest_toml`, parse with `toml::Value`, assert `parsed.get("sessions").and_then(|v| v.as_array()).is_some()` and `parsed.get("session").is_none()`. Comment: "Assay's RunManifest uses `sessions` (plural); any regression to `session` causes a silent parse failure."
4. Write **`test_run_manifest_spec_is_sanitized_name_not_description`**: session `name = "unit-tests"`, `spec = "Run the unit test suite"`, `harness = "cargo test"`. Assert `sessions[0]["spec"].as_str().unwrap() == "unit-tests"` (sanitized name) and `sessions[0].get("spec").unwrap().as_str().unwrap() != "Run the unit test suite"` (not the description).
5. Write **`test_run_manifest_no_unknown_fields`**: create a manifest, call `build_run_manifest_toml`, attempt `toml::from_str::<SmeltRunManifest>(&toml_str)` — must succeed. Also check `sessions[0].get("harness").is_none()` and `sessions[0].get("timeout").is_none()` on the raw `toml::Value`.
6. Write **`test_spec_toml_structure`**: call `build_spec_toml` on a session with `name = "auth"`, `spec = "Implement the auth flow"`, `harness = "cargo test --test auth"`. Parse with `toml::Value`. Assert: `parsed["name"].as_str().unwrap() == "auth"`, `parsed["description"].as_str().unwrap() == "Implement the auth flow"`, `parsed["criteria"].as_array().unwrap().len() >= 1`, `criteria[0]["cmd"].as_str().unwrap() == "cargo test --test auth"`.
7. Write **`test_spec_toml_deny_unknown_fields_roundtrip`**: call `build_spec_toml`, then `toml::from_str::<SmeltSpec>` — must succeed without error (if Smelt emits an unknown field, this test catches the regression).
8. Write **`test_sanitize_session_name`** (table-driven using a `cases` vec of `(&str, &str)` tuples):
   - `("frontend", "frontend")` — already clean
   - `("my/session", "my-session")` — slash replaced
   - `("my session", "my-session")` — space replaced
   - `("a/b/c", "a-b-c")` — multi-slash
   - `("trailing-", "trailing")` — trailing dash trimmed
   - `("-leading", "leading")` — leading dash trimmed
   - `("", "unnamed")` — empty → fallback
   - `("---", "unnamed")` — all dashes → fallback after trim
   Loop: `for (input, expected) in cases { assert_eq!(AssayInvoker::sanitize_session_name(input), expected, "input = {input:?}"); }`
9. Write **`test_build_run_command_includes_base_branch`**: build manifest with `base_ref = "main"`. Call `build_run_command`. Find index of `"--base-branch"` in returned vec. Assert next element equals `"main"`.
10. Write **`test_build_run_command_includes_timeout`**: build manifest with two sessions, timeouts 300 and 900. Assert `"--timeout"` in vec and next element equals `"900"` (max).
11. Write **`test_build_ensure_specs_dir_command`**: assert `build_ensure_specs_dir_command() == vec!["mkdir", "-p", "/workspace/.assay/specs"]`.
12. Write **`test_build_write_assay_config_command`**: call `build_write_assay_config_command("my-project")`. Assert: `cmd[0] == "sh"`, `cmd[1] == "-c"`, `cmd[2]` contains `"if [ ! -f /workspace/.assay/config.toml ]"`, `cmd[2]` contains `"base64 -d"`, `cmd[2]` contains `"/workspace/.assay/config.toml"`.
13. Write **`test_multi_session_depends_on_preserved`**: three-session manifest with `depends_on`. Assert `sessions[2]["depends_on"].as_array().unwrap()` contains expected names; `sessions[0].get("depends_on").is_none()` (skip_serializing_if).
14. Run `cargo test -p smelt-core 2>&1`. All tests must pass.
15. Run `cargo test --workspace 2>&1 | tail -20`. No new failures introduced.

## Must-Haves

- [ ] `test_run_manifest_uses_sessions_key_plural` passes
- [ ] `test_run_manifest_spec_is_sanitized_name_not_description` passes
- [ ] `test_run_manifest_no_unknown_fields` passes — including deny_unknown_fields roundtrip
- [ ] `test_spec_toml_structure` passes — name, description, criteria[0].cmd all correct
- [ ] `test_sanitize_session_name` passes all table rows including empty/all-dashes fallback
- [ ] `test_build_run_command_includes_base_branch` passes
- [ ] `test_build_write_assay_config_command` passes — idempotency guard present in sh -c string
- [ ] `cargo test -p smelt-core` shows `0 failed`
- [ ] `cargo test --workspace` introduces no new failures

## Verification

```bash
cd /Users/wollax/Git/personal/smelt
cargo test -p smelt-core 2>&1 | tail -10
# Expected: "test result: ok. N passed; 0 failed; 0 ignored"
```

```bash
cargo test --workspace 2>&1 | grep -E "^test result|FAILED|error\[" | tail -20
# No "FAILED" lines for tests that were passing before S01
```

## Observability Impact

- Signals added/changed: None — tests exercise pure functions; no runtime signals
- How a future agent inspects this: `cargo test -p smelt-core -- --nocapture` prints tracing logs for each test run; test names are descriptive and identify which contract invariant failed
- Failure state exposed: test failure messages include the specific TOML content that violated the assertion (via `assert!` messages with `?` formatting)

## Inputs

- `crates/smelt-core/src/assay.rs` — post-T01 state: all four types and eight methods must exist; no test module yet
- `crates/smelt-core/src/manifest.rs` — `JobManifest::from_str` for the test helper
- S01-RESEARCH.md — exact field names, deny_unknown_fields rules, sanitization edge cases

## Expected Output

- `crates/smelt-core/src/assay.rs` — complete `#[cfg(test)] mod tests` block with 13+ named unit tests
- `cargo test -p smelt-core` green
- `cargo test --workspace` no new failures
