# S01: Manifest Extension — Research

**Researched:** 2026-03-21
**Domain:** Rust serde / TOML manifest extension
**Confidence:** HIGH

## Summary

S01 adds `ComposeService` and `Vec<ComposeService>` to `JobManifest`, extends `validate()` with compose-specific rules, and proves the schema is correct with unit tests. All the technical ingredients are already present in the codebase — `toml::Value`, serde, and the existing validation pattern in `manifest.rs`. The only new dependency is `indexmap` for deterministic YAML output (D073).

The primary implementation challenge is choosing the right serde design for `ComposeService`. The boundary map specifies `#[serde(flatten)] extra: IndexMap<String, toml::Value>` to capture arbitrary Compose service fields beyond `name` and `image`. Serde's flatten behavior with `IndexMap` is: named fields consume their keys first; the `IndexMap` captures all remaining keys. This means `name` and `image` do NOT appear in `extra`, which is the correct behavior. `ComposeService` must NOT have `deny_unknown_fields` — intentional passthrough design (D073).

The slice is entirely within `smelt-core/src/manifest.rs` plus a new `#[serde(default)] services: Vec<ComposeService>` field on `JobManifest`. No `run.rs` changes are needed for S01 (dry-run works because the dry-run path has no runtime-dispatch check; the `execute_run` Phase 3 check will be updated in S04).

## Recommendation

Use `#[derive(Debug, Deserialize)] pub struct ComposeService` with explicit `name: String` and `image: String` fields, plus `#[serde(flatten)] pub extra: IndexMap<String, toml::Value>`. Add `#[serde(default)] pub services: Vec<ComposeService>` to `JobManifest`. Do NOT add `deny_unknown_fields` to `ComposeService`.

Add `indexmap` to workspace deps at `1.x` (latest stable). It is already a transitive dependency of many crates — adding it explicitly gives version control.

Validation additions:
1. All existing validation untouched
2. If `runtime != "compose"` and `!services.is_empty()` → error
3. If `runtime == "compose"`: for each service entry, validate `name` is non-empty and `image` is non-empty
4. `runtime` value validation: must be "docker" or "compose" (catches typos early)

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Arbitrary-field passthrough in serde | `#[serde(flatten)] IndexMap<K, V>` | Standard serde pattern; captures all keys not consumed by named fields |
| Validation error aggregation | `Vec<String>` errors pattern in `manifest.rs:validate()` | Already established (D018); just push more error strings |
| TOML parsing of nested tables/arrays | `toml::Value` (recursive enum) | Already in workspace deps; handles arrays, inline tables, integers, booleans |

## Existing Code and Patterns

- `crates/smelt-core/src/manifest.rs` — `JobManifest` with `#[serde(deny_unknown_fields)]` on all structs and `validate()` with error aggregation. Add `services: Vec<ComposeService>` to `JobManifest` as a new field with `#[serde(default)]`. Add compose validation in the existing `validate()` body.
- `crates/smelt-core/src/manifest.rs::validate()` — Error collection pattern: `errors.push(format!(...))` then `Err(SmeltError::Manifest { ... })` at the end if non-empty. Follow exactly for new service validation errors.
- `crates/smelt-core/Cargo.toml` — `serde_yaml` not yet present; `indexmap` not yet present. Both must be added. `serde_yaml` is a production dep (used in S02 `generate_compose_file()`); `indexmap` is also production.
- `crates/smelt-cli/src/commands/run.rs::print_execution_plan()` — will need a `── Compose Services ──` section in S04; no S01 changes needed here.
- `crates/smelt-cli/src/commands/run.rs` Phase 3 — currently rejects `runtime != "docker"`. S01 does NOT change this; `--dry-run` bypasses Phase 3. The dispatch logic is updated in S04.

## Constraints

- `deny_unknown_fields` is on `JobManifest` (D017), so `services` MUST be added as an explicit named field — it cannot be unknown.
- `ComposeService` must NOT have `deny_unknown_fields` — this would reject the passthrough fields.
- `#[serde(default)]` on `services` is required so existing manifests without `[[services]]` continue to parse (backward compat).
- `toml` v1.0.6 (current lock) supports `#[serde(flatten)]` with `IndexMap` — verified by baseline test run (121 pass). No version bump needed.
- `IndexMap` requires explicit `Deserialize` derive support; the `indexmap` crate provides this via `serde` feature: `indexmap = { version = "2", features = ["serde"] }`.
- `toml::Value` contains the full TOML type enum including `Table` (for nested objects like `healthcheck`), `Array`, `Integer`, `Float`, `Boolean`, `String`, `Datetime`. Passthrough to YAML in S02 must handle all of these.

## Common Pitfalls

- **`deny_unknown_fields` + `#[serde(flatten)]` interaction** — Adding `deny_unknown_fields` to `ComposeService` AND `#[serde(flatten)]` on its extra field is a known serde incompatibility; it causes the flattened map to never receive keys. The fix is: do not put `deny_unknown_fields` on `ComposeService`. This is intentional per D073.

- **`name` appearing in `extra`** — Serde's flatten behavior: named struct fields are deserialized first and consume their keys; the flattened map receives only REMAINING keys. So `name` and `image` will NOT be in `extra`. This is correct behavior — verify in roundtrip test by asserting `extra` does NOT contain `"name"` or `"image"`.

- **`runtime` validation** — Currently `validate()` does not check the runtime value (only non-emptiness). The Phase 3 check in `run.rs` catches unknown runtimes at execution time. For S01, we add validation in `validate()` that restricts runtime to `["docker", "compose"]`. This is the right place per D018 (collect all errors upfront). Verify that the existing `VALID_MANIFEST` test constant uses `runtime = "docker"` — it does, so no existing tests break.

- **`[[services]]` TOML array-of-tables syntax** — In TOML, `[[services]]` creates an array of tables. Serde maps this to `Vec<ComposeService>` on the `services` field. There's no ambiguity, but the field name in `JobManifest` must exactly match `services` (the TOML key). ✓

- **Zero services with `runtime = "compose"`** — Allowed per the context doc: "What happens if `runtime = "compose"` but `[[services]]` is empty? — Allow it". Do NOT add a validation error for empty services list when runtime is compose.

## Open Risks

- **`toml::Value` + `serde(flatten)` edge case for empty inline tables** — An entry like `environment = {}` in a `[[services]]` block may deserialize to `toml::Value::Table(BTreeMap::new())`. This should serialize to YAML correctly in S02, but worth confirming in S02 snapshot tests.

- **`indexmap` version** — D073 says `IndexMap<String, toml::Value>` but doesn't specify the indexmap crate version. Version 2.x is the current stable; it has serde support behind `features = ["serde"]`. Verify no conflicts with existing transitive deps (none expected since indexmap 2.x is backward-compatible with many crates).

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust / serde | — | no specific skill needed |

## Sources

- Codebase analysis: `crates/smelt-core/src/manifest.rs` (existing validation pattern, struct layout, deny_unknown_fields usage)
- Codebase analysis: `crates/smelt-core/Cargo.toml` (dep list — no indexmap, no serde_yaml yet)
- Codebase analysis: `Cargo.toml` workspace (dep list — toml v1.0.6 confirmed)
- Decision log: D017 (deny_unknown_fields), D018 (collect-all-errors), D073 (IndexMap passthrough), D076 (serde_yaml production dep)
- M004-ROADMAP.md boundary map: S01→S02 interface spec
- Baseline test run: `cargo test -p smelt-core` — 121 passed, 0 failed (no regressions before any changes)
