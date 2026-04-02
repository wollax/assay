---
estimated_steps: 6
estimated_files: 4
---

# T01: Extend JobManifest with forge config and RunState with PR fields

**Slice:** S02 — Manifest Forge Config + PR Creation
**Milestone:** M003

## Description

Add `forge: Option<ForgeConfig>` to `JobManifest` with proper serde wiring so `[forge]` is accepted in TOML manifests and absent when the section is omitted. Extend `validate()` to collect forge-specific errors. Add `pr_url` and `pr_number` to `RunState` with `#[serde(default)]` for backward compatibility. Add `--no-pr` flag to `RunArgs`. Enable the `forge` feature in smelt-cli's Cargo.toml. Write tests covering all these changes before T02 wires the runtime behavior.

## Steps

1. **`manifest.rs` — import ForgeConfig and add the field**: Add `use crate::forge::ForgeConfig;` to the imports. Add `#[serde(default)] pub forge: Option<ForgeConfig>` to `JobManifest`. The struct already has `#[serde(deny_unknown_fields)]` — this attribute applies to `JobManifest`'s own unknown keys; `ForgeConfig` has its own `deny_unknown_fields`. No structural change needed.

2. **`manifest.rs` — add forge validation in `validate()`**: After the merge section, add:
   ```rust
   if let Some(ref forge) = self.forge {
       if forge.token_env.trim().is_empty() {
           errors.push("forge.token_env: must not be empty".to_string());
       }
       let valid_repo = forge.repo.split_once('/').map(|(owner, name)| !owner.is_empty() && !name.is_empty()).unwrap_or(false);
       if !valid_repo {
           errors.push(format!("forge.repo: must be in `owner/repo` format, got `{}`", forge.repo));
       }
   }
   ```
   Do NOT call into `forge.rs` private helpers. Do NOT check if the env var is actually set (structural validation only — D018 + research constraint).

3. **`monitor.rs` — add PR fields to RunState**: Add two fields with `#[serde(default)]` on each individually:
   ```rust
   #[serde(default)]
   pub pr_url: Option<String>,
   #[serde(default)]
   pub pr_number: Option<u64>,
   ```
   Placement: after `pid: u32`. The `#[serde(default)]` on each field (not on the struct) is required so existing state files written without these fields deserialize without error.

4. **`run.rs` — add `--no-pr` flag to `RunArgs`**: Add `#[arg(long)] pub no_pr: bool` to `RunArgs`. No default annotation needed — clap defaults bool args to false.

5. **`smelt-cli/Cargo.toml` — enable forge feature**: Change `smelt-core.path = "../smelt-core"` to `smelt-core = { path = "../smelt-core", features = ["forge"] }`.

6. **Tests in `manifest.rs`**: Add the following tests in the `#[cfg(test)]` block:
   - `test_parse_manifest_with_forge` — parse a TOML with `[forge]` section; assert `manifest.forge.is_some()`, `forge.provider == "github"`, `forge.repo == "owner/my-repo"`, `forge.token_env == "GITHUB_TOKEN"`
   - `test_parse_manifest_without_forge` — parse VALID_MANIFEST (no `[forge]`); assert `manifest.forge.is_none()`
   - `test_validate_forge_invalid_repo_format` — `forge.repo = "no-slash"` → validation error contains "owner/repo format"
   - `test_validate_forge_empty_token_env` — `forge.token_env = ""` → validation error contains "forge.token_env: must not be empty"
   - `test_forge_deny_unknown_fields` — `[forge]` with an unknown field → parse error (deny_unknown_fields)

   Add in `monitor.rs` tests:
   - `test_run_state_backward_compat_no_pr_fields` — serialize a RunState TOML without `pr_url`/`pr_number` fields (manually construct the string), deserialize it; assert `pr_url.is_none()`, `pr_number.is_none()`

## Must-Haves

- [ ] `cargo test -p smelt-core` passes — all pre-existing tests plus the 6 new tests
- [ ] `cargo build --workspace` compiles cleanly with forge feature enabled in smelt-cli
- [ ] `manifest.forge.is_none()` when no `[forge]` section in TOML (explicit test)
- [ ] `manifest.forge.is_some()` with correct fields when `[forge]` present (explicit test)
- [ ] Forge validation errors collected into existing `errors` vec (not returned early)
- [ ] Old RunState TOML without `pr_url`/`pr_number` deserializes successfully (backward-compat test)
- [ ] `--no-pr` flag accepted by clap without error (verified by `cargo build`)

## Verification

- `cargo test -p smelt-core -- manifest forge` shows 5+ new tests passing
- `cargo test -p smelt-core -- monitor` shows backward-compat test passing
- `cargo build --workspace` compiles clean
- `cargo test -p smelt-core` shows all 118+ pre-existing tests still pass (no regressions)

## Observability Impact

- Signals added/changed: `RunState.pr_url` and `RunState.pr_number` are written to `.smelt/run-state.toml` — a future agent running `smelt status` can read them; once S03 renders the PR section, these fields drive it
- How a future agent inspects this: `cat .smelt/run-state.toml | grep pr_url` after a run; T02 writes the values; this task establishes the schema
- Failure state exposed: forge validation errors surface via the same `SmeltError::Manifest` path as all other validation errors — printed to stderr with all errors together (D018)

## Inputs

- `crates/smelt-core/src/forge.rs` — `ForgeConfig` struct already exported unconditionally (D055); import via `use crate::forge::ForgeConfig` in `manifest.rs`
- `crates/smelt-core/src/manifest.rs` — existing `validate()` pattern: push to `errors` Vec, check `errors.is_empty()` at end
- `crates/smelt-core/src/monitor.rs` — existing `RunState` struct; `#[serde(default)]` per-field pattern established by `#[serde(default)] resources: HashMap` in `Environment`
- S01 summary — D055 confirms ForgeConfig is unconditional; no forge feature needed in smelt-core for manifest.rs

## Expected Output

- `crates/smelt-core/src/manifest.rs` — `JobManifest` has `forge: Option<ForgeConfig>`; `validate()` includes forge checks; 5 new tests covering parse and validation
- `crates/smelt-core/src/monitor.rs` — `RunState` has `pr_url: Option<String>`, `pr_number: Option<u64>` with `#[serde(default)]`; 1 new backward-compat test
- `crates/smelt-cli/src/commands/run.rs` — `RunArgs` has `no_pr: bool`
- `crates/smelt-cli/Cargo.toml` — smelt-core dep includes `features = ["forge"]`
