# S01: Fix AssayInvoker — Real Assay Contract — Research

**Date:** 2026-03-17

## Summary

S01 is a pure unit-test slice: replace `AssayInvoker`'s broken serde types and methods with ones whose generated TOML passes Assay's `deny_unknown_fields` guards — provable without Docker. The three contract violations are all in `crates/smelt-core/src/assay.rs`:

1. **`[[session]]` → `[[sessions]]`**: `AssayManifest { session: Vec<AssaySession> }` serializes to the wrong TOML key. Real `RunManifest` expects `sessions` (plural). Both the struct field name and the generated TOML must change.
2. **Inline `spec` description → spec-name reference**: `AssaySession.spec` currently holds a free-text description (`SessionDef.spec`). Real `ManifestSession.spec` is a name reference to `.assay/specs/<name>.toml`. The description must move into a separately generated spec file.
3. **Unknown fields `harness` / `timeout`**: Both `RunManifest` and `ManifestSession` use `deny_unknown_fields`. Any extra field silently causes a TOML parse error. `harness` becomes a `[[criteria]]` `cmd` inside the spec file; `timeout` moves to the `--timeout` flag on `assay run`.

The solution is **Option A** (confirmed by D043/D044/D045): Smelt generates both spec files and the `RunManifest`. No dependency on `assay-types` crate (D002). New Smelt-side serde structs mirror the real schema, verified against source.

The existing tests in `assay.rs` test the wrong types and must be replaced wholesale. The `run.rs` wiring (Phase 5.5) is S02 scope — S01 only delivers the `AssayInvoker` API and its unit tests.

## Recommendation

Replace `AssayManifest`/`AssaySession` with four new Smelt-local serde structs:

```rust
// Mirrors assay-types RunManifest — deny_unknown_fields
struct SmeltRunManifest { sessions: Vec<SmeltManifestSession> }

// Mirrors assay-types ManifestSession — only fields Smelt uses
struct SmeltManifestSession { spec: String, name: Option<String>, depends_on: Vec<String> }

// Mirrors assay-types Spec — deny_unknown_fields on source
struct SmeltSpec { name: String, description: String, criteria: Vec<SmeltCriterion> }

// Mirrors assay-types Criterion — only name/description/cmd needed
struct SmeltCriterion { name: String, description: String, cmd: Option<String> }
```

New `AssayInvoker` methods to add (all pure/infallible unless noted):

| Method | Purpose |
|--------|---------|
| `build_run_manifest_toml(manifest)` | Replaces `build_manifest_toml()`; uses `sessions` key; `spec` = sanitized name |
| `build_spec_toml(session)` | Generates a flat `Spec` TOML for one `SessionDef` |
| `sanitize_session_name(name)` | Replaces `/`, spaces, and non-`[a-zA-Z0-9_-]` with `-`; trims leading/trailing `-` |
| `build_ensure_specs_dir_command()` | `["mkdir", "-p", "/workspace/.assay/specs"]` |
| `build_write_assay_config_command(project_name)` | Idempotent: writes `/workspace/.assay/config.toml` only if it doesn't exist |
| `write_spec_file_to_container(provider, container, name, toml)` | Base64 exec write to `/workspace/.assay/specs/<name>.toml` — mirrors `write_manifest_to_container()` |
| `build_run_command(manifest)` (extend) | Add `--base-branch <manifest.job.base_ref>` |

Rename `build_manifest_toml` → `build_run_manifest_toml` (breaking rename, caught at compile time in `run.rs`).

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Base64 file write into container | `write_manifest_to_container()` pattern (D028) | Proven; `write_spec_file_to_container()` is a direct copy parameterized on path |
| TOML serialization of new structs | `toml::to_string_pretty()` — already a dep | No new crate needed |
| Idempotent directory creation | `mkdir -p` in shell command (D044) | Already used for other setup; always safe to repeat |
| Session name uniqueness guarantee | `JobManifest::validate()` already enforces unique names | Sanitized names won't collide if original names don't |

## Existing Code and Patterns

- `crates/smelt-core/src/assay.rs` — **rewrite entirely**: `AssayManifest`/`AssaySession` types → 4 new types; `build_manifest_toml()` → `build_run_manifest_toml()`; `write_manifest_to_container()` is reusable as-is; all existing tests reference wrong types and must be replaced
- `crates/smelt-core/src/manifest.rs` — `SessionDef` has `name`, `spec` (description), `harness`, `timeout`, `depends_on`; **no changes needed**: `spec` stays as description on Smelt's side; translation happens entirely in `AssayInvoker`
- `crates/smelt-cli/src/commands/run.rs` — Phase 6 calls `build_manifest_toml()` — rename becomes a compile error pointing to the exact line to update; Phase 5.5 wiring (spec writes) is S02 scope but the renamed method must compile
- `crates/smelt-cli/tests/docker_lifecycle.rs` — mock assay tests use the old TOML format (via `write_manifest_to_container`); those tests will continue to work as long as the mock assay binary doesn't validate TOML schema — no S01 changes needed here

## Authoritative Contract (verified from source)

### `RunManifest` → `SmeltRunManifest`
```
Source: /Users/wollax/Git/personal/assay/crates/assay-types/src/manifest.rs
deny_unknown_fields: YES
Required fields: sessions (Vec)
```
```toml
[[sessions]]
spec = "session-name"           # name reference — REQUIRED
name = "optional display name"  # optional
depends_on = ["other"]          # optional, default empty
```
Extra fields `harness`, `timeout` → **parse fail** (deny_unknown_fields).

### `Spec` → `SmeltSpec`
```
Source: /Users/wollax/Git/personal/assay/crates/assay-types/src/lib.rs (Spec struct)
deny_unknown_fields: YES
Required fields: name (String), criteria (Vec<Criterion>)
Optional: description (default ""), gate (Option<GateSection>), depends (Vec)
```
```toml
name = "session-name"
description = "free-text from SessionDef.spec"

[[criteria]]
name = "harness"
description = "Harness gate for <session-name>"
cmd = "npm test"
```
Note: `criteria` has **no `#[serde(default)]`** — it's required in the struct but can be an empty vec. Always include at least one criterion (the harness cmd) so the gate is meaningful.

### `Criterion` → `SmeltCriterion`
```
Source: /Users/wollax/Git/personal/assay/crates/assay-types/src/criterion.rs
deny_unknown_fields: YES
Required fields: name (String), description (String)
Optional: cmd, path, timeout, enforcement, kind, prompt, requirements
```
Smelt only needs `name`, `description`, `cmd`. The others are `skip_serializing_if = "Option::is_none"` so safe to omit in output.

### `GateSection` → not needed
The `[gate]` section is optional and defaults to `enforcement = "required"`. Smelt can omit it entirely — criteria default to required enforcement.

### `assay run` flags
```
--timeout <secs>       already present; keep using max session timeout
--base-branch <branch> ADD: manifest.job.base_ref
```

## Constraints

- **D002 (firm):** No `assay-types` import — define local mirror types with `deny_unknown_fields` on the Smelt side too (catches Smelt's own regressions)
- **`deny_unknown_fields` on both sides:** Add `#[serde(deny_unknown_fields)]` to all four new structs; if they diverge from Assay's schema, Smelt's own unit tests catch it before the integration test runs
- **`criteria` is non-optional in `Spec`:** A session with no harness would produce a `SmeltSpec` with an empty `criteria` vec — Assay may accept this (it's a `Vec`, not validated as non-empty by serde), but it produces a spec with no gates. The current `SessionDef` always has `harness: String` (required, no default), so this won't happen in practice
- **`sanitize_session_name` scope:** Session names are already validated as unique in `JobManifest::validate()`. Sanitization only affects filesystem safety (slash, space, etc.) — it cannot introduce collisions for manifests that pass validation
- **No `run.rs` phase-5.5 wiring in S01:** S01 delivers the `AssayInvoker` API and unit tests only; `run.rs` continues to call `build_manifest_toml()` → rename to `build_run_manifest_toml()` is the only compile-time change required; spec writes are wired in S02

## Common Pitfalls

- **Forgetting to add `#[serde(deny_unknown_fields)]` to Smelt's mirror types** — without it, Smelt's own round-trip tests can pass even if the generated TOML has extra fields; add it as a regression guard
- **Leaving `session` (singular) field anywhere** — the struct field name determines the TOML key; `SmeltRunManifest { session: ... }` would still serialize to `[[session]]`; must be `sessions`
- **`--base-branch` on `build_run_command`** — `manifest.job.base_ref` is the correct source; don't use `manifest.merge.target`
- **`write_spec_file_to_container` path** — must write to `/workspace/.assay/specs/<sanitized-name>.toml`, not `/tmp/.assay/specs/`; Assay resolves specs relative to project root (D027 = `/workspace`)
- **Existing tests assert `[[session]]`** — the current test `test_manifest_toml_is_valid_toml` deserializes back to `AssayManifest` and checks `roundtrip.session.len()` — this confirms the old wrong format; all existing tests in `assay.rs` must be deleted/replaced, not extended

## Open Risks

- **`criteria` empty vec behavior in Assay** — if `SessionDef.harness` is blank (which validation currently allows via non-empty check missing), Smelt would generate `criteria = []`. This is a latent edge case; the immediate risk is low because S01 tests use non-empty harness strings. Add a note for future validation hardening.
- **Sanitized spec name vs. `ManifestSession.spec` mismatch** — `build_run_manifest_toml()` must use the *same* sanitization as `build_spec_toml()`. If they diverge (e.g., one trims trailing `-`, one doesn't), Assay won't find the spec file. The sanitization must be a single shared function called from both methods.
- **`SmeltManifestSession.name` vs `spec`** — The `name` field in `ManifestSession` is an *optional display name*; `spec` is the file reference. Smelt should set `spec = sanitized_name` and `name = Some(session.name.clone())` (original unsanitized) for display; if the original name equals the sanitized name, omit `name` (or always include it — both are valid).

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust / Cargo | — | none found (standard) |
| TOML serde | — | none found (standard) |

## Sources

- `RunManifest` and `ManifestSession` with `deny_unknown_fields` (source: `/Users/wollax/Git/personal/assay/crates/assay-types/src/manifest.rs`)
- `Spec` struct with `deny_unknown_fields`, `criteria: Vec<Criterion>` required (source: `/Users/wollax/Git/personal/assay/crates/assay-types/src/lib.rs`)
- `Criterion` struct with `deny_unknown_fields`, required `name`+`description`, optional `cmd` (source: `/Users/wollax/Git/personal/assay/crates/assay-types/src/criterion.rs`)
- `GateSection` and `Enforcement` enum (kebab-case: `required`/`advisory`) (source: `/Users/wollax/Git/personal/assay/crates/assay-types/src/enforcement.rs`)
- Real spec example with `[[criteria]]` and `cmd` fields (source: `/Users/wollax/Git/personal/assay/.assay/specs/self-check.toml`)
- Current (wrong) `AssayInvoker` implementation — `session` key, inline spec, `harness`/`timeout` fields (source: `crates/smelt-core/src/assay.rs`)
- `SessionDef` — `spec` is free-text description, `harness` is always present string (source: `crates/smelt-core/src/manifest.rs`)
- Current `run.rs` Phase 6 calls `build_manifest_toml()` — compile error on rename (source: `crates/smelt-cli/src/commands/run.rs`)
