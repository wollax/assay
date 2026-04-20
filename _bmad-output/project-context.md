---
project_name: 'assay'
user_name: 'Wollax'
date: '2026-04-20'
sections_completed:
  ['technology_stack', 'language_rules', 'testing_rules', 'quality_rules', 'workflow_rules', 'critical_rules']
status: 'complete'
rule_count: 95
optimized_for_llm: true
---

# Project Context for AI Agents

_This file contains critical rules and patterns that AI agents must follow when implementing code in this project. Focus on unobvious details that agents might otherwise miss._

---

## Technology Stack & Versions

- **Language:** Rust (stable channel, Edition 2024)
- **Workspace version:** 0.5.0
- **Build tools:** `just` (justfile), `cargo-nextest`, `cargo-deny`
- **MCP:** rmcp 0.17 (server, transport-io)
- **Async:** tokio 1 (full), tower 0.5
- **HTTP:** axum 0.8, reqwest 0.13
- **TUI:** ratatui 0.30, crossterm 0.28
- **CLI:** clap 4 (derive)
- **Serialization:** serde 1, serde_json 1, toml 1, schemars 1
- **Telemetry:** tracing 0.1, opentelemetry 0.31, tracing-opentelemetry 0.32
- **Errors:** thiserror 2, color-eyre 0.6
- **Context engine:** cupel 1.2.0
- **Infrastructure:** bollard 0.20 (Docker)
- **Testing:** insta 1.46 (json snapshots), tempfile 3, assert_cmd 2

## Critical Implementation Rules

### Language-Specific Rules (Rust)

- **Crate placement:** New serializable types → `assay-types`; business logic → `assay-core`; MCP tools → `assay-mcp`; state backends → `assay-backends`; CLI/TUI are thin wrappers delegating to core only.
- **Dependency DAG:** `types ← core ← {backends, harness, mcp, cli, tui}`. Types never depend on core. Core never depends on downstream crates. Cycles are a hard error.
- **Error crate split:** `thiserror` in library crates for typed errors; `color-eyre` only in binary crates for user-facing reports. Never use `eyre::Result` in library code.
- **Public type derives in `assay-types`:** `Debug, Clone, Serialize, Deserialize, JsonSchema` — always pair `PartialEq` with `Eq` (clippy `deny_partial_eq_without_eq` enforced).
- **Error types:** `thiserror::Error` with `#[non_exhaustive]` on all error enums. Matches across crate boundaries require a `_` wildcard arm; adding variants requires auditing all match sites.
- **Result alias:** Each crate defines `pub type Result<T> = std::result::Result<T, CrateError>;`
- **Sync-first core (D007):** No `async fn` in `assay-core` or `assay-types` — period. Async surface lives in `assay-backends`, `assay-mcp`, `assay-cli`, `assay-tui`, `smelt-*`. Any call into `assay-core` from async context must use `tokio::task::spawn_blocking` — no exceptions.
- **Visibility discipline:** Default to `pub(crate)` in `assay-core`. Only make items `pub` if they're part of the crate's API and re-exported in `lib.rs`.
- **`#![deny(missing_docs)]`** in `assay-types` and `assay-core`. Doc comments must add information not already in the name. Style: imperative mood for functions (`/// Resolve the spec path`), period at end.
- **Module `//!` blocks** must cover: what the module does, what it doesn't (boundaries), and invariants callers must respect.
- **Workspace dependencies only:** All deps in root `Cargo.toml` `[workspace.dependencies]`, crates inherit with `workspace = true`. Never add version numbers in individual crate `Cargo.toml`.
- **Re-export flattening:** `pub use` at crate root in `lib.rs`. Re-exports use `#[doc(inline)]` — never add a divergent second doc comment.
- **Inventory schema registry:** Types with `JsonSchema` submit via `inventory::submit!()`. Registrations are link-time — registry tests must live in integration test binaries, not unit tests.
- **Serde conventions:** `#[serde(deny_unknown_fields)]` on config types; `#[serde(default, skip_serializing_if = "Option::is_none")]` for optionals; tag-based enums. **Never combine `deny_unknown_fields` with `#[serde(flatten)]`** — they are incompatible.
- **Backward compat aliases:** `#[serde(alias = "OldName")]` when renaming variants for existing data compatibility.
- **Schemars 1 (not 0.8):** API surface differs from pre-1.0. Use `schemars::schema_for!` for testing. Default to `///` doc comment extraction for schema descriptions; reserve explicit `#[schemars(description = "...")]` only for MCP tool param structs where LLM-facing phrasing differs from developer docs.
- **Clippy rules:** `cognitive-complexity-threshold = 25` — decompose functions before commit if exceeded.
- **Edition 2024:** Avoid `gen` as an identifier (reserved keyword).
- **Test runner:** `cargo nextest run` — not `cargo test`. Nextest has stricter isolation and different timeout behavior.
- **Schema regeneration:** After adding/modifying types in the schema registry, run `just schemas` and commit the updated `schemas/*.json` files. CI enforces via `schemas-check`.
- **Feature-gated code:** Test with `--all-features` to ensure `deny(missing_docs)` and clippy catch items behind cfg gates.
- **MCP error handling:** Never propagate domain errors with `?` in tool handlers. Catch `assay-core` errors and convert to descriptive `CallToolResult { is_error: true }`. Reserve `McpError` for infrastructure failures only.
- **Version bumps:** After changing `workspace.package.version`, run `just sync-plugin-version` to keep plugin.json in sync.

### Testing Rules

- **Two test categories:** *Implementation tests* (internal correctness) and *contract tests* (LLM consumer guarantees). Contract tests cover MCP tool response shapes, gate evaluation reasons, and session error payloads — changes to these are breaking changes requiring deliberate review.
- **Test placement by crate:** `assay-types` = unit tests only; `assay-core` = co-located unit tests + `tests/` only for multi-concept workflows; `assay-backends` = unit for adapter logic, feature-gated `tests/` for real external APIs; `assay-mcp`/`assay-cli`/`assay-tui` = `tests/` for end-to-end invocation.
- **Unit tests inline, integration in `tests/`:** Never put integration test helpers in `src/`.
- **Test naming:** File names mirror what they test. Functions use `test_` prefix with behavior-focused names.
- **MCP tool testing:** Extract handler logic into sync core functions and test those directly. MCP layer gets 1-2 integration tests verifying JSON-RPC shape, not business logic. Snapshot MCP response shapes as contract artifacts.
- **Never test sync logic through async tests.** Test sync `assay-core` functions directly. Only use `#[tokio::test]` for code that's actually async.
- **Insta snapshots:** Snapshots are source artifacts. `INSTA_UPDATE=no` in CI. Run `cargo insta review` locally, commit `.snap` files. Snapshot only at public API/output boundaries — never internal state.
- **Fixture pattern:** Bind `TempDir` to a named variable — never inline `TempDir::new().unwrap().path()`. The TempDir must live to the closing brace of the test.
- **`#[serial]` scope:** Any test touching `std::env` needs `#[serial_test::serial]`. Only serializes within the same test binary — cross-crate shared state needs file locks or env var guards.
- **`#[traced_test]` is the subscriber:** Never call `tracing_subscriber::init()` inside a `#[traced_test]` body — it panics on double-init.
- **CLI tests:** `assert_cmd::Command` must assert specific stderr content for error paths, not just exit code.
- **Docker tests:** `docker_lifecycle` tests skip gracefully when Docker unavailable. Use `just test-smelt-unit` to exclude explicitly.
- **Core tests use in-memory data, not mock backends.** Mocking a backend in core tests means your dependency arrow is wrong.
- **Gate evaluations:** Test boundary precision — assert `reason` strings and failure detail, not just `passed: bool`. LLMs act on reason content.
- **Session state transitions:** Test every valid AND invalid transition. Assert error payload structure for invalid moves.
- **Spec resolution:** Test ambiguous inputs (partial IDs, aliases, archived specs) for deterministic behavior.

### Code Quality & Style Rules

- **Quality gate:** `just ready` runs all checks (fmt-check, lint, test, deny, check-plugin-version). Run before considering work complete. The justfile is the contract — if a gate is worth running, it belongs in `just ready`.
- **Git hooks:** `.githooks/` — pre-commit runs fmt-check + lint + check-plugin-version; pre-push adds full test suite + deny. Set up with `just setup`. Never bypass with `--no-verify`.
- **Clippy is deny-level:** `-- -D warnings`. No `#[allow(clippy::*)]` without a justifying comment. Enable `unreachable_pub` — agents over-publicize.
- **Single-definition rule:** No domain concept defined in more than one crate. Before adding a type to any crate, search `assay-types` first. Duplication is a bug.
- **Types in `assay-types` must have consumers:** No type without at least one non-test use site in another crate. Don't add speculative types "for later."
- **Traits in `assay-types` are minimal:** Data shapes and serialization contracts only. Smart logic belongs in `assay-core`.
- **Error message ownership:** Error types own their formatting via `#[error]` attributes. Wrapping sites add context, never re-format the inner error.
- **No `unwrap()` in library code:** Use `?` or explicit error handling. `unwrap()`/`expect()` only in tests, examples, and binary entry points.
- **Import ordering:** std → external crates → workspace crates → local modules, separated by blank lines.
- **File naming:** `snake_case.rs`. Complex domains use directory with `mod.rs` + sub-files.
- **No magic literals:** Numeric constants in business logic must be named with `const`.
- **Prefer borrowing in return position:** Return `&str` or `Cow<str>` over owned `String` unless ownership transfer is necessary.
- **Module size trigger:** >400 lines signals review for splitting.
- **`cargo deny`:** Enforces license allowlist, security advisories, and dependency bans. New deps may require `deny.toml` updates.
- **Doc comments serve three audiences:** Rustdoc readers, schemars-generated JSON Schema, and LLM tool consumers. Write for the most demanding (LLM + schema) — if an LLM reads only the doc comment, can it call the function correctly?
- **Inline `//` comments explain *why*, not *what*.** No restating the code.
- **`// SAFETY:` mandatory for any `unsafe` block** — explain which invariant is upheld.
- **Marker convention:** `// TODO(username): description` (known gap), `// FIXME(username): description` (known bug), `// HACK(reason): description` (intentional shortcut — must name what it sacrifices).
- **Feature flags must be documented** in the crate's top-level `//!` doc comment.
- **Test helpers used by >1 crate** belong in a shared test support module, not duplicated.

### Development Workflow Rules

- **Branch workflow:** Feature branches off `main`. Push to Forgejo at `forgejo.alexwollan.com` (SSH port 2222), not GitHub. No force-push to main.
- **Commit conventions:** Conventional commits — `feat(crate):`, `fix(crate):`, `chore:`, `docs:`, `refactor:`, `test:`. Scope is the crate name for crate-local changes. 72-char subject, no period. Breaking changes get `!` suffix and `BREAKING CHANGE:` footer.
- **MCP API breaking changes:** Tool additions are non-breaking. Tool removals or signature changes are breaking → require major version bump and `BREAKING CHANGE:` footer.
- **Atomic cross-crate changes:** `just build` must pass on every committed ref. Breaking changes to public API must include downstream fixes in the same commit/PR.
- **Pre-push failure:** Run `just ready` locally, read the error, fix root cause, create new commit. Never `--no-verify`.
- **Adding new dependencies:** Add to `[workspace.dependencies]` in root `Cargo.toml` first, then reference as `{ workspace = true }`. Feature flags at workspace level. Run `cargo deny check` after.
- **Adding new crates:** Place in `crates/` (assay) or `smelt/crates/` (infra). Declare in `[workspace.members]` same commit. Deps via workspace. Must have `lib.rs` `//!` doc.
- **Cross-project verification:** Changes to `assay-types` require `just build-smelt`. Mandatory.
- **Version bumps:** Standalone commit. Bump `workspace.package.version` → `just sync-plugin-version` → commit atomically.
- **Release process:** Tags are `vX.Y.Z`, pushed explicitly. Tags trigger GitHub Actions release. Never create tags without passing `just ready` + human checkpoint.
- **Schema artifacts:** After changing registry types, run `just schemas` and commit `schemas/*.json` alongside the code.
- **Git hooks setup:** `just setup` once after clone.

### Critical Don't-Miss Rules

**Architectural Invariants:**
- **No tokio in `assay-core` or `assay-types` — at all.** No imports, no futures, no async trait methods, no `Handle::current()`. The sync boundary (D007) is absolute.
- **Never hold `std::sync::RwLock` or `Mutex` guards across `.await`** in MCP handlers. Causes timing-dependent deadlocks invisible in tests. Clone data out or use `tokio::sync::` equivalents.
- **`deny_unknown_fields` + `#[serde(flatten)]` = runtime panic.** Never combine. Serde limitation.
- **Feature-gated `inventory::submit!`** — if the feature isn't enabled in the final binary, the registration silently disappears.

**MCP & LLM Contract Rules:**
- **MCP tool response string shape IS the public API.** Exact strings, field names, and envelope structure require deprecation policy. LLMs branch on string content.
- **Gate evaluation ordering is semantic, not cosmetic.** Parallel/shuffled evaluation can produce vacuous successes.
- **Phase completion must be a function of gate results, not a flag.**

**Serde/Schemars Traps:**
- **`#[serde(default)]` silently swallows missing fields.** Only add when you explicitly want fallback — never to "make it compile."
- **Schema ≠ deserialization behavior.** Round-trip test any schema used as a contract.
- **`schemars` 1 ≠ 0.8.** Verify attribute syntax against workspace version.

**Testing & Runtime Gotchas:**
- **`std::env::set_var` is UB in multithreaded processes.** Any env-mutating test needs `#[serial]`.
- **Use port `0` for OS-assigned ports** in integration tests. Hardcoded ports collide under nextest shards.
- **`#[serial]` only serializes within a single test binary.** Cross-crate shared state needs file locks.
- **Per-crate green ≠ ready.** Only workspace-wide `just ready` is the safe gate.
- **`#[derive(Default)]` on enums** silently picks the first variant. Use explicit `impl Default` if that's wrong.

**Domain Invariants:**
- **Active spec implies a live session holds it.** Dead session + active spec = zombie.
- **Archiving with failed gates** silently ships unmet criteria. Require all gates passed or explicit bypass reason.

**Distribution & Release:**
- **Plugin version desync breaks distribution.** `just sync-plugin-version` after any version change.
- **Agents should never create git tags.** Tags trigger release workflows. Human checkpoint required.
- **Stale schemas break CI.** Touch a `JsonSchema` type → run `just schemas` → commit results.

---

## Usage Guidelines

**For AI Agents:**
- Read this file before implementing any code
- Follow ALL rules exactly as documented
- When in doubt, prefer the more restrictive option
- Update this file if new patterns emerge

**For Humans:**
- Keep this file lean and focused on agent needs
- Update when technology stack changes
- Review quarterly for outdated rules
- Remove rules that become obvious over time

Last Updated: 2026-04-20
