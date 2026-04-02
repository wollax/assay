# S02: TrackerSource Trait, Config, & Template Manifest — Research

**Date:** 2026-03-27
**Domain:** Rust async traits, TOML serde, template-based manifest generation
**Confidence:** HIGH

## Summary

S02 establishes the foundational abstractions for tracker-driven dispatch: the `TrackerSource` trait, `TrackerConfig` in `ServerConfig`, template manifest loading/validation, issue injection into `[[session]]` entries, a `MockTrackerSource` for testing, and the `JobSource::Tracker` variant. This is a pure contract + unit test slice — no real GitHub/Linear API calls.

The codebase has strong, consistent patterns for every piece needed. `ForgeConfig` (token_env env var pattern), `ForgeClient` trait (RPITIT, generic at callsites), `ServerConfig` (deny_unknown_fields + validation), `JobManifest` (deny_unknown_fields + semantic validation), and `SmeltError` (operation+message convention) all provide direct templates to follow. The `state_backend` passthrough (D154) mirrors Assay's `StateBackendConfig` enum — a Smelt-side serde struct with no crate dependency.

The primary risk is trait design: the roadmap says `TrackerSource` should be object-safe for `Arc<dyn TrackerSource>`, but the existing codebase uses RPITIT (non-object-safe) for all traits. S02 must decide: object-safe with `#[async_trait]` / `Pin<Box<dyn Future>>`, or non-object-safe with generics at callsites (matching D019/D121). Since S05 will store the tracker in `dispatch_loop` (which already uses generics), the generic approach is viable and consistent. However, D150 (one tracker per instance) means the concrete type is known at startup — `Box<dyn TrackerSource>` via manual vtable or enum dispatch (like `AnyProvider` in D084) are also options.

## Recommendation

Follow the existing codebase patterns exactly:

1. **Trait design:** Use RPITIT (non-object-safe) like `ForgeClient`/`RuntimeProvider`/`SshClient`. Use generics at callsites. If S05 needs dynamic dispatch, use an enum wrapper (D084 `AnyProvider` pattern) — not `dyn`.

2. **Config:** Add `tracker: Option<TrackerConfig>` to `ServerConfig` with `deny_unknown_fields`. `TrackerConfig` follows `ForgeConfig` pattern: env var names for secrets, not values. Validation in `ServerConfig::validate()`.

3. **Template manifest:** Load and validate at startup via `JobManifest::load()` + `validate()`. Store the parsed `JobManifest` in `TrackerConfig` or alongside it. At dispatch time, clone the template and inject `[[session]]` entries from the issue.

4. **New types in smelt-core:** `TrackerIssue` struct, `StateBackendConfig` mirror enum. Add to `manifest/mod.rs` or a new `tracker.rs` module.

5. **New types in smelt-cli:** `TrackerConfig` in `serve/config.rs`, `TrackerSource` trait + `MockTrackerSource` in a new `serve/tracker.rs` module.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Async trait with Send bounds | RPITIT (Rust 2024 edition, D019) | Already used everywhere; no `async_trait` crate needed |
| Template TOML loading | `JobManifest::load()` + `validate()` | Reuse exact parsing + validation pipeline |
| Env var secret indirection | `ForgeConfig.token_env` pattern | Proven pattern (D014, D112); never store values in config |
| Error type for tracker ops | `SmeltError::Tracker { operation, message }` | Follows `SmeltError::Forge` pattern exactly |
| Mock for testing | `MockSshClient` pattern in `ssh/mock.rs` | VecDeque-based response queue; proven in 12+ test sites |
| State backend config | Assay's `StateBackendConfig` enum | Mirror as Smelt-side serde struct per D002/D154 |

## Existing Code and Patterns

- `crates/smelt-core/src/forge.rs` — `ForgeConfig` with `deny_unknown_fields`, `token_env: String` pattern; `ForgeClient` trait with RPITIT. Direct template for `TrackerSource` trait design.
- `crates/smelt-cli/src/serve/config.rs` — `ServerConfig` with `deny_unknown_fields`, `Option<AuthConfig>` pattern for optional sections, `validate()` with collected errors (D018). Direct template for `Option<TrackerConfig>`.
- `crates/smelt-core/src/manifest/mod.rs` — `JobManifest` with `deny_unknown_fields`, `session: Vec<SessionDef>`. Template manifest is a `JobManifest` loaded from a separate file; issue injection appends to `session` vec.
- `crates/smelt-core/src/manifest/validation.rs` — `validate_manifest()` with collected errors. Add tracker-specific validation rules here or in `ServerConfig::validate()`.
- `crates/smelt-core/src/error.rs` — `SmeltError` enum with `#[non_exhaustive]`, convenience constructors. Add `Tracker { operation, message }` variant following `Forge` pattern.
- `crates/smelt-cli/src/serve/types.rs` — `JobSource` enum with `serde(rename_all = "snake_case")`. Add `Tracker` variant.
- `crates/smelt-cli/src/serve/dispatch.rs` — `dispatch_loop` generic over `SshClient`. Same pattern for `TrackerSource` in S05.
- `crates/smelt-cli/src/serve/ssh/mod.rs` — `SshClient` trait + `MockSshClient`. Direct template for `TrackerSource` trait + `MockTrackerSource`.
- `crates/smelt-core/src/assay.rs` — `AssayInvoker::build_run_manifest_toml()` translates `JobManifest` → Assay TOML. Template manifest + issue injection produces a `JobManifest` that goes through the same pipeline.
- `../assay/crates/assay-types/src/state_backend.rs` — `StateBackendConfig` enum with `rename_all = "snake_case"`, explicit `#[serde(rename = "github")]` for `GitHub` variant. Mirror this in `smelt-core` per D154.

## Constraints

- **`deny_unknown_fields` on `ServerConfig`** — new `tracker` field must be added to the struct or TOML parsing will fail for configs that include `[tracker]`.
- **`deny_unknown_fields` on `JobManifest`** — template manifests with a `[state_backend]` field will fail to parse unless the field is added to `JobManifest`. This is S05 work but the template manifest loading path in S02 must account for it.
- **D002 — no Assay crate dependency** — `StateBackendConfig` must be a Smelt-side serde struct, not imported from Assay.
- **D017 — strict manifest parsing** — template manifest validated at `ServerConfig::load()` time, not at dispatch time.
- **D018 — collect all errors** — tracker validation must collect all errors, not fail-fast.
- **D150 — polling, not webhooks** — `TrackerSource::poll_ready_issues()` returns a batch; no webhook handler.
- **D151 — one tracker per instance** — `[tracker]` is a single section, not `[[tracker]]` array.
- **D152 — template + injection model** — template provides infrastructure; issue provides session content.
- **D153 — label-based lifecycle** — `TrackerState` enum maps to label names, not platform-specific states.
- **D155 — GitHub uses `gh` CLI** — trait must not assume HTTP-only; `gh` subprocess is a valid impl.
- **D156 — Linear uses reqwest** — trait must not assume subprocess; HTTP client is a valid impl.

## Common Pitfalls

- **Template manifest with `[[session]]` entries** — If the template already has sessions, injection should append, not replace. But the roadmap says "template provides environment/credentials/merge; each issue injects sessions." Safer to require the template has zero sessions and validate this at startup. Avoids ambiguity.
- **`state_backend` field on `JobManifest`** — Adding this field now (S02) would require updating the init skeleton (D065), all test helpers, and dry-run output. Better to add it in S05 when the passthrough is actually wired. S02 can define the `StateBackendConfig` type but not add it to `JobManifest` yet.
- **Trait method signatures too specific** — `poll_ready_issues()` should return `Vec<TrackerIssue>`, not platform-specific types. `transition_state()` takes `(issue_id, from_state, to_state)` to enable the atomic label swap (D157). `issue_to_manifest()` is NOT a trait method — it's a free function that takes a template + issue and produces a `JobManifest`, since the logic is identical for all backends.
- **TOML session field name** — `JobManifest` uses `session: Vec<SessionDef>` (singular key for TOML `[[session]]`). Injection must use `session` not `sessions`. The Assay run manifest uses `sessions` (plural) — don't confuse the two.
- **serde `rename_all` on `StateBackendConfig`** — Assay uses `rename_all = "snake_case"` which turns `GitHub` into `git_hub`, requiring an explicit `#[serde(rename = "github")]`. Must replicate this in the mirror type.

## Open Risks

- **Template manifest forward-compatibility** — When S05 adds `[state_backend]` to `JobManifest`, template manifests written for S02 won't have it. This is fine — it's `Option`. But templates that DO include `[state_backend]` before S05 will fail to parse (deny_unknown_fields). Acceptable: we document that `[state_backend]` is added in S05.
- **`issue_to_manifest` session construction** — The roadmap says "issue title → session name, body → spec text." But `SessionDef` also requires `harness` and `timeout`. These must come from somewhere — likely additional fields on `TrackerConfig` (e.g. `default_harness`, `default_timeout`). Not mentioned in roadmap — must decide during planning.
- **Object-safety decision** — If S05 needs `Arc<dyn TrackerSource>` (e.g. stored in ServerState), the RPITIT trait won't work. The `AnyProvider` enum dispatch pattern (D084) is the fallback. This is S05's problem, but trait design in S02 should not paint S05 into a corner.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust/Cargo | N/A | Core language — no skill needed |
| serde/TOML | N/A | Already deeply used in codebase |
| tokio async | N/A | Already deeply used in codebase |

No external skills are relevant — this is pure Rust systems programming using patterns already established in the codebase.

## Sources

- Codebase: `crates/smelt-core/src/forge.rs` — ForgeConfig/ForgeClient pattern (HIGH confidence)
- Codebase: `crates/smelt-cli/src/serve/config.rs` — ServerConfig pattern (HIGH confidence)
- Codebase: `crates/smelt-core/src/manifest/mod.rs` — JobManifest parsing pattern (HIGH confidence)
- Codebase: `crates/smelt-cli/src/serve/ssh/mod.rs` — SshClient trait + mock pattern (HIGH confidence)
- Codebase: `../assay/crates/assay-types/src/state_backend.rs` — StateBackendConfig enum to mirror (HIGH confidence)
- Decisions register: D019, D084, D121, D150-D157 — architectural constraints (HIGH confidence)
