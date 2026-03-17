# S06: RunManifest Type & Parsing — Research

**Date:** 2026-03-16

## Summary

S06 delivers the `RunManifest` type system and TOML parsing for declarative run manifests. The codebase has well-established patterns for TOML-backed types (config, specs) with `deny_unknown_fields`, schema snapshots, and two-phase parse-then-validate error reporting. The main design question is the `[[sessions]]` entry type name — `SessionEntry` is already taken by the JSONL context parser in `assay-types/src/context.rs`, so the manifest session type must use a different name (e.g., `ManifestSession`).

The work is straightforward: define types in `assay-types/src/manifest.rs`, parsing + validation in `assay-core/src/manifest.rs`, follow the existing config/spec patterns exactly, and add schema snapshot tests. Low risk as stated in the roadmap.

## Recommendation

Follow the existing config module pattern exactly:

1. **Types** in `assay-types/src/manifest.rs`: `RunManifest` (top-level) and `ManifestSession` (per-session entry). Use `[[sessions]]` TOML array per D004. All standard derives + `deny_unknown_fields` + `inventory::submit!`.
2. **Parsing** in `assay-core/src/manifest.rs`: `from_str()` returning `toml::de::Error`, `load()` returning `Result<RunManifest>` with `format_toml_error()` for actionable messages, and `validate()` returning `Vec<ManifestError>` for semantic checks.
3. **Error variants** in `assay-core/src/error.rs`: `ManifestParse` and `ManifestValidation`, following `ConfigParse`/`ConfigValidation` pattern.
4. **Schema snapshots** in `assay-types/tests/schema_snapshots.rs` for `RunManifest` and `ManifestSession`.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| TOML deserialization | `toml` crate (0.8, workspace dep) | Already used for config + specs, handles `[[array]]` natively |
| Error formatting with line/column | `config::format_toml_error()` | Production-quality caret-pointer error display, tested |
| Schema snapshot locking | `insta` + `schemars` pattern in `schema_snapshots.rs` | Identical pattern used for all 32+ existing snapshots |
| Semantic validation | Two-phase pattern: `from_str()` then `validate()` | Separates syntax errors from business rule errors cleanly |

## Existing Code and Patterns

- `crates/assay-core/src/config/mod.rs` — canonical TOML parsing pattern: `from_str()` → `load()` with `format_toml_error()` → `validate()` returning `Vec<ConfigError>`. Reuse `format_toml_error()` directly (make it `pub(crate)` or duplicate).
- `crates/assay-types/src/harness.rs` — `HarnessProfile`, `SettingsOverride`, `PromptLayer`, `HookContract` types that `ManifestSession` references. `SettingsOverride` fields are all `Option`/`Vec` with skip_serializing_if — embed directly in session.
- `crates/assay-types/src/lib.rs` — re-export pattern: `pub mod manifest` + `pub use manifest::{RunManifest, ManifestSession}`.
- `crates/assay-types/tests/schema_snapshots.rs` — add `run_manifest_schema_snapshot()` and `manifest_session_schema_snapshot()` following identical pattern.
- `crates/assay-core/src/error.rs` — `ConfigParse { path, message }` and `ConfigValidation { path, errors }` patterns to mirror for `ManifestParse`/`ManifestValidation`.
- `crates/assay-core/src/lib.rs` — add `pub mod manifest` following existing module list pattern.

## Constraints

- **`SessionEntry` name is taken** — `assay-types/src/context.rs` exports `pub enum SessionEntry` (JSONL context entries). The manifest session type must use a different name. `ManifestSession` is clear and unambiguous.
- **`deny_unknown_fields` required** — all persisted structs in the codebase use it; manifest types must too.
- **`inventory::submit!` required** — all types with `JsonSchema` register themselves in the schema registry.
- **`toml` crate is dev-dependency only in assay-types** — TOML round-trip tests go in dev, but runtime parsing goes in assay-core (which has `toml` as a regular dependency).
- **Forward compatibility for multi-agent (D004)** — `[[sessions]]` array even for single-session, so multi-session in M002 doesn't break the format.
- **Zero-trait convention (D001)** — parsing functions are plain functions, not trait methods.

## Common Pitfalls

- **Embedding HarnessProfile directly in ManifestSession** — HarnessProfile has `name` and `working_dir` which are session-level concerns, not harness-level in the manifest context. Better to have ManifestSession contain the relevant harness fields (settings, hooks, prompt_layers) as optional inline overrides, plus a `spec` field pointing to the spec. The pipeline (S07) constructs a full HarnessProfile from manifest + spec + defaults.
- **Forgetting to re-export from lib.rs** — every public type in assay-types must be re-exported from `lib.rs` for downstream crates to use.
- **Overly strict validation** — validate required fields (spec, at least one session) but don't over-constrain optional harness overrides. S07 will resolve defaults at pipeline time.
- **Not testing TOML-specific features** — `[[sessions]]` array syntax, inline tables, multiline strings in prompt content. Test with realistic TOML, not just minimal examples.

## Open Risks

- **ManifestSession → HarnessProfile mapping complexity** — the exact fields on `ManifestSession` determine how much work S07 needs to do to construct a `HarnessProfile`. Keep the manifest type close to what the user writes (declarative, minimal) and let S07 handle the transformation. If unsure, keep fields optional.
- **`format_toml_error` visibility** — currently `pub(crate)` in config module. The manifest module in assay-core needs it too. Either make it a shared utility or duplicate the small function.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust | `personamanagmentlayer/pcl@rust-expert` | available (38 installs) — general Rust, not specifically relevant |
| TOML / serde | none | no relevant skill found |

No skills are directly relevant to this slice's work. The codebase patterns are well-established and self-documenting.

## Sources

- Existing codebase patterns (source: `crates/assay-core/src/config/mod.rs`, `crates/assay-types/src/harness.rs`)
- S02 summary (source: `.kata/milestones/M001/slices/S02/S02-SUMMARY.md`)
- Decisions register (source: `.kata/DECISIONS.md` — D001, D004, D006)
- Boundary map (source: `.kata/milestones/M001/M001-ROADMAP.md` — S06 → S07 contract)
