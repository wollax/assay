---
id: T01
parent: S05
milestone: M003
provides:
  - smelt-core Cargo.toml has keywords, categories, and homepage metadata
  - lib.rs crate-level doc has two paragraphs and a no_run usage example
  - SmeltRunManifest, SmeltManifestSession, SmeltSpec, SmeltCriterion are pub(crate)
  - AssayInvoker and all its pub fn methods remain pub
  - smelt-cli compiles clean after visibility demotion
key_files:
  - crates/smelt-core/Cargo.toml
  - crates/smelt-core/src/lib.rs
  - crates/smelt-core/src/assay.rs
key_decisions:
  - "JobManifest::from_str takes two args (content: &str, source: &Path) — usage example in lib.rs updated accordingly"
patterns_established:
  - "Internal Assay serde translation types use pub(crate) — they are not part of the embedding API"
observability_surfaces:
  - "cargo doc --no-deps -p smelt-core — crate landing page now shows description and usage example"
duration: 10min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T01: Cargo metadata, lib.rs doc, and assay type API curation

**Added Cargo publishing metadata, a two-paragraph crate-level doc with `no_run` usage example, and demoted four internal Assay translation structs to `pub(crate)` — smelt-cli compiles clean, all tests pass.**

## What Happened

Three targeted, non-behavioral changes to set the foundation for T02/T03:

1. **Cargo.toml** — appended `keywords`, `categories`, and `homepage` after the `description` field. The crate is now publish-ready from a metadata standpoint.

2. **lib.rs** — replaced the single `//!` line with a full crate-level doc block: an introductory paragraph explaining the library's purpose, a feature-flag paragraph describing the `forge` gate, and a `# Example` section with a `no_run` block showing `JobManifest::from_str` and `GitHubForge::new`. One deviation: the initial draft called `from_str("...")` with one argument, but the actual signature is `from_str(content: &str, source: &Path)` — the example was updated to pass `std::path::Path::new("smelt.toml")` as the second argument so the doctest compiles.

3. **assay.rs** — changed all four internal serde translation types (`SmeltRunManifest`, `SmeltManifestSession`, `SmeltSpec`, `SmeltCriterion`) from `pub` to `pub(crate)`. `AssayInvoker` and all its `pub fn` methods were left untouched. smelt-cli has no references to these types by name, so compilation succeeded without further changes.

## Verification

```
# Cargo metadata
grep "keywords" crates/smelt-core/Cargo.toml    → keywords = [...]
grep "categories" crates/smelt-core/Cargo.toml  → categories = [...]
grep "homepage" crates/smelt-core/Cargo.toml    → homepage = "https://..."

# pub(crate) structs
grep -n "pub.*struct Smelt" crates/smelt-core/src/assay.rs
# 31:pub(crate) struct SmeltRunManifest
# 42:pub(crate) struct SmeltManifestSession
# 60:pub(crate) struct SmeltSpec
# 75:pub(crate) struct SmeltCriterion

# AssayInvoker still pub
grep "^pub struct AssayInvoker" crates/smelt-core/src/assay.rs → pub struct AssayInvoker;

# cargo build -p smelt-cli → 0 errors
# cargo test -p smelt-core -q → test result: ok. 3 passed; 0 failed
# cargo doc --no-deps -p smelt-core → 0 errors
```

## Diagnostics

`cargo doc --no-deps -p smelt-core --open` — the crate landing page now shows the description and usage example. No runtime signals added (purely additive metadata/doc/visibility changes).

## Deviations

`JobManifest::from_str` requires a second `source: &Path` argument. The task plan example only passed one argument. Fixed in lib.rs to call `from_str("...", std::path::Path::new("smelt.toml"))` so the doctest compiles and passes.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/Cargo.toml` — added `keywords`, `categories`, `homepage`
- `crates/smelt-core/src/lib.rs` — expanded crate-level `//!` doc with paragraphs and `no_run` example
- `crates/smelt-core/src/assay.rs` — `SmeltRunManifest`, `SmeltManifestSession`, `SmeltSpec`, `SmeltCriterion` changed to `pub(crate)`
