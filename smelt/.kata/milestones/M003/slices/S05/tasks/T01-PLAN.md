---
estimated_steps: 5
estimated_files: 3
---

# T01: Cargo metadata, lib.rs doc, and assay type API curation

**Slice:** S05 — smelt-core Library API
**Milestone:** M003

## Description

Three lightweight changes that set the foundation for T02/T03: (1) add Cargo publishing metadata (keywords, categories, homepage) so the crate is publish-ready; (2) expand the crate-level `//!` doc in `lib.rs` with a multi-paragraph description and a `no_run` usage example; (3) demote the four Assay-internal translation structs (`SmeltRunManifest`, `SmeltManifestSession`, `SmeltSpec`, `SmeltCriterion`) from `pub` to `pub(crate)` to remove them from the embedding API surface. None of these changes affect runtime behavior or test logic.

## Steps

1. **Cargo.toml metadata** — open `crates/smelt-core/Cargo.toml` and add after the `description` field:
   ```toml
   keywords = ["docker", "ci", "github", "assay", "infrastructure"]
   categories = ["command-line-utilities", "development-tools"]
   homepage = "https://github.com/wollax/smelt"
   ```

2. **lib.rs crate doc** — replace the single `//! Smelt core library — …` line with a multi-line doc block:
   - First paragraph: what the library is and why it exists
   - Second paragraph: feature flags (forge gates octocrab)
   - A `# Example` heading with a ` ```rust,no_run ``` ` block showing:
     - `use smelt_core::{JobManifest, SmeltConfig};`
     - `#[cfg(feature = "forge")] use smelt_core::forge::GitHubForge;`
     - A two-line example constructing a `GitHubForge` client and parsing a manifest

3. **assay.rs demotion** — change `pub struct SmeltRunManifest`, `pub struct SmeltManifestSession`, `pub struct SmeltSpec`, `pub struct SmeltCriterion` to `pub(crate) struct …`. Do NOT change `pub struct AssayInvoker` or any of its `pub fn` methods. These four structs are internal translation types only used inside `AssayInvoker` function bodies; smelt-cli never references them by type name.

4. **Compile check** — run `cargo build -p smelt-cli` to confirm smelt-cli still compiles after the visibility change. Then run `cargo test -p smelt-core -q` to confirm no smelt-core test regressions.

5. **Docs check** — run `cargo doc --no-deps -p smelt-core` (without `RUSTDOCFLAGS`) to confirm docs generate without error (missing_docs won't be denied yet — that's T03).

## Must-Haves

- [ ] `smelt-core/Cargo.toml` has `keywords`, `categories`, `homepage` fields
- [ ] `lib.rs` `//!` doc has at least two paragraphs and a `no_run` code example
- [ ] `SmeltRunManifest`, `SmeltManifestSession`, `SmeltSpec`, `SmeltCriterion` are `pub(crate)` in `assay.rs`
- [ ] `AssayInvoker` and all its `pub fn` methods remain `pub`
- [ ] `cargo build -p smelt-cli` succeeds (no compilation errors from the demotion)
- [ ] `cargo test -p smelt-core -q` passes

## Verification

```bash
# Cargo metadata present
grep "keywords" crates/smelt-core/Cargo.toml
grep "categories" crates/smelt-core/Cargo.toml
grep "homepage" crates/smelt-core/Cargo.toml

# Assay types are pub(crate), AssayInvoker is still pub
grep "pub.crate. struct Smelt" crates/smelt-core/src/assay.rs
grep "^pub struct AssayInvoker" crates/smelt-core/src/assay.rs

# smelt-cli compiles
cargo build -p smelt-cli 2>&1 | grep -E "^error" | head -5

# smelt-core tests pass
cargo test -p smelt-core -q 2>&1 | tail -3
```

## Observability Impact

- Signals added/changed: None — no runtime behavior changes
- How a future agent inspects this: `cargo doc --no-deps -p smelt-core --open` — the crate landing page now has a description and usage example
- Failure state exposed: None — this task is purely additive

## Inputs

- `crates/smelt-core/Cargo.toml` — existing; add metadata fields
- `crates/smelt-core/src/lib.rs` — existing `//!` single-line doc; expand it
- `crates/smelt-core/src/assay.rs` — existing; demote four internal structs

## Expected Output

- `crates/smelt-core/Cargo.toml` — has `keywords`, `categories`, `homepage`
- `crates/smelt-core/src/lib.rs` — crate doc has usage example in `no_run` block
- `crates/smelt-core/src/assay.rs` — `SmeltRunManifest` et al. are `pub(crate)`; smelt-cli compiles clean
