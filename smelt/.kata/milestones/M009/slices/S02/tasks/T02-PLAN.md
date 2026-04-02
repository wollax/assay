---
estimated_steps: 5
estimated_files: 7
---

# T02: Annotate all 7 example manifests with field-level comments

**Slice:** S02 — README + example manifest documentation
**Milestone:** M009

## Description

Add inline `#` comments to all 7 example TOML files explaining every field's purpose, valid values, and defaults. Fix the broken `agent-manifest.toml` which uses `[manifest]` instead of `[job]` (fails parsing due to D017 `deny_unknown_fields`). Cross-reference all field names against the serde structs in `manifest.rs` and `config.rs` to ensure accuracy. Verify all parseable examples still work after annotation.

## Steps

1. Fix `agent-manifest.toml`: change `[manifest]` to `[job]`, verify it parses with `smelt run examples/agent-manifest.toml --dry-run`
2. Annotate `agent-manifest.toml` — add header comment explaining this is a minimal agent-style manifest, comment each field (name, base_ref, session fields including task/file_scope/timeout_secs)
3. Annotate `job-manifest-k8s.toml` — add header and field-level comments for all sections including `[kubernetes]` block fields (namespace, ssh_key_env, cpu_request, memory_request, cpu_limit, memory_limit). Cross-reference against `KubernetesConfig` struct in `manifest.rs`
4. Annotate remaining examples: expand comments on `job-manifest.toml` (already partially commented), `job-manifest-compose.toml` (already partially commented), `job-manifest-forge.toml` (already partially commented); for `bad-manifest.toml` add a comment above each intentional error explaining what validation rule it violates; for `server.toml` verify existing comments are complete and expand if needed
5. Verify all 6 valid examples pass `--dry-run`; verify `bad-manifest.toml` fails; verify each file has substantial comment coverage (at minimum: agent ≥5, k8s ≥10, bad ≥8 comment lines)

## Must-Haves

- [ ] `agent-manifest.toml` uses `[job]` instead of `[manifest]` and parses successfully
- [ ] All 7 example files have `#` comments on every field explaining purpose and valid values
- [ ] `bad-manifest.toml` documents each intentional error with a comment
- [ ] `job-manifest-k8s.toml` has comments on all `[kubernetes]` fields
- [ ] All 6 valid examples pass `smelt run <file> --dry-run`
- [ ] `bad-manifest.toml` exits non-zero on `--dry-run`
- [ ] `cargo test --workspace` passes — no regressions

## Verification

- `cargo run -- run examples/agent-manifest.toml --dry-run` — exits 0
- `cargo run -- run examples/job-manifest-k8s.toml --dry-run` — exits 0
- `cargo run -- run examples/job-manifest.toml --dry-run` — exits 0
- `cargo run -- run examples/job-manifest-forge.toml --dry-run` — exits 0
- `cargo run -- run examples/job-manifest-compose.toml --dry-run` — exits 0
- `cargo run -- run examples/bad-manifest.toml --dry-run` — exits non-zero
- `grep -c '^#' examples/agent-manifest.toml` — ≥5
- `grep -c '^#' examples/job-manifest-k8s.toml` — ≥10
- `cargo test --workspace` — all tests pass

## Observability Impact

- Signals added/changed: None
- How a future agent inspects this: Read example files directly
- Failure state exposed: None

## Inputs

- `crates/smelt-core/src/manifest.rs` — `JobManifest`, `JobMeta`, `Environment`, `CredentialConfig`, `SessionDef`, `MergeConfig`, `ComposeService`, `KubernetesConfig` struct definitions (field names, types, serde attributes)
- `crates/smelt-cli/src/serve/config.rs` — `ServerConfig` struct definition
- All 7 existing example files in `examples/`

## Expected Output

- `examples/agent-manifest.toml` — fixed `[job]` key + full field-level comments
- `examples/bad-manifest.toml` — error-explaining comments on each intentional violation
- `examples/job-manifest.toml` — expanded field-level comments
- `examples/job-manifest-compose.toml` — expanded field-level comments including `[[services]]` block
- `examples/job-manifest-forge.toml` — expanded field-level comments including `[forge]` block
- `examples/job-manifest-k8s.toml` — full field-level comments including `[kubernetes]` block
- `examples/server.toml` — verified/expanded comments
