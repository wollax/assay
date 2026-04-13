# Phase 65: Resolution Core - Research

**Researched:** 2026-04-11
**Domain:** Rust / assay-core domain logic — criteria library I/O and gate resolution
**Confidence:** HIGH

## Summary

Phase 65 adds two tightly scoped capabilities to `assay-core`: (1) criteria library file I/O (load/save/scan from `.assay/criteria/<slug>.toml`) mirroring existing patterns from `spec::scan` and `save_session`, and (2) a pure `spec::compose::resolve()` function that merges parent gate criteria, library criteria, and own criteria into a `ResolvedGate` carrying per-criterion source tracking, with cycle detection and slug validation. All types live in `assay-types` for MCP tool access (Phase 68).

The codebase supplies every pattern this phase needs in already-written, already-tested form. No new dependencies are required. The implementation is pure plumbing — composing existing infrastructure into new entry points.

**Primary recommendation:** Copy-adapt existing patterns (`scan`, `load_gates`, `save_session`/`NamedTempFile`) exactly. Do not invent new conventions.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- `ResolvedCriterion` and `ResolvedGate` types live in `assay-types` (not assay-core) — MCP tools (Phase 68) need to return them
- `ResolvedGate` struct wraps criteria list with metadata: parent name, included libraries
- All new types get schemars `JsonSchema` derives immediately
- Closure pair: `load_gate: impl Fn(&str) -> Result<GatesSpec>` and `load_library: impl Fn(&str) -> Result<CriteriaLibrary>` — lazy loading, zero-trait convention
- Single-level extends only: A extends B, B's extends is ignored. Max chain depth 2. Multi-level deferred to INHR-05.
- Child's `include` libraries only — parent's includes are not re-resolved
- Lives in `spec::compose` module (`crates/assay-core/src/spec/compose.rs`)
- New `validate_slug()` function: pattern `^[a-z0-9][a-z0-9_-]*$`, max 64 chars — allows hyphens AND underscores
- Applied to: library slugs, extends values, include values — AND retroactively to spec slugs and gate names
- `pub fn` visibility — downstream phases need slug validation
- Replaces `validate_path_component` for slug contexts (path_component stays for non-slug identifiers)
- Structured per-error AssayError variants: `CycleDetected`, `LibraryNotFound`, `ParentGateNotFound`, `InvalidSlug`
- Fail on first error (not collecting all errors)
- Fuzzy suggestions on not-found errors — reuse existing fuzzy matching pattern from gate/spec errors

### Claude's Discretion

- Source annotation detail level (simple enum vs origin + override chain)
- Whether precondition command output reuses existing head+tail truncation
- Exact serde field ordering and skip_serializing_if patterns (follow existing conventions)
- Library I/O atomicity (tempfile-then-rename pattern from work_session, or simpler direct write)

### Deferred Ideas (OUT OF SCOPE)

- Multi-error collection in resolve() — future milestone
- Multi-level inheritance INHR-05 — already in REQUIREMENTS.md Future section
- Parameterized/template criteria CLIB-05 — already in REQUIREMENTS.md Future section
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| INHR-03 | Circular `extends` chains are detected and reported as validation errors | `resolve()` receives both gate slugs at depth 2; trivial to detect: if `extends == self` or parent also extends back to child, emit `CycleDetected`. Existing `levenshtein`/`find_fuzzy_match` in `spec/mod.rs` supports suggestions on the not-found path. |
| INHR-04 | Gate run output shows per-criterion source annotation (parent vs own) | `ResolvedCriterion` wraps `Criterion` with a `CriterionSource` enum; `ResolvedGate` is the resolved product. Both types live in `assay-types`. Source annotation approach (simple enum) is sufficient for single-level. |
| CLIB-01 | User can define shared criteria sets in `.assay/criteria/<slug>.toml` | `CriteriaLibrary` type already exists in `assay-types`. Library file I/O functions (`load_library`, `save_library`, `scan_libraries`) follow the `load_gates`/`spec::scan` pattern. |
| CLIB-02 | User can reference criteria libraries via `include` field in gate definitions | `GatesSpec.include: Vec<String>` already exists. `resolve()` uses the `load_library` closure to load each included slug and merges criteria in order. Slug validation via `validate_slug()` guards the include values. |
| CLIB-03 | Core API supports load, save, and scan operations for criteria libraries | Three free functions in a new `spec::criteria` (or inline `spec::compose`) module: `load_library(path) -> Result<CriteriaLibrary>`, `save_library(assay_dir, lib) -> Result<PathBuf>` (atomic), `scan_libraries(assay_dir) -> Result<Vec<CriteriaLibrary>>`. |
</phase_requirements>

## Standard Stack

### Core (no new dependencies)

| Library | Already Used | Purpose | Note |
|---------|-------------|---------|------|
| `toml` | workspace dep | TOML parse/serialize for `.assay/criteria/*.toml` | Same as `load_gates` |
| `tempfile` | workspace dep | Atomic file writes via `NamedTempFile` | Same pattern as `save_session` |
| `schemars` | workspace dep | `JsonSchema` derive on `ResolvedGate`, `ResolvedCriterion`, `CriterionSource` | Mandatory for all assay-types |
| `serde` | workspace dep | `Serialize`/`Deserialize` on all new types | Standard |
| `thiserror` | workspace dep | New `AssayError` variants | Already in error.rs |
| `inventory` | workspace dep | Schema registration for new assay-types | `inventory::submit!` block needed |

No new Cargo.toml entries required.

## Architecture Patterns

### New File Layout

```
crates/
  assay-types/src/
    resolved_gate.rs       # ResolvedGate, ResolvedCriterion, CriterionSource (new)
    lib.rs                 # pub mod resolved_gate; pub use resolved_gate::*;
  assay-core/src/
    spec/
      compose.rs           # resolve(), validate_slug() (new submodule)
      mod.rs               # pub mod compose; + scan_libraries / load/save_library fns
```

### Pattern 1: Library I/O — mirror `load_gates` / `save_session`

**What:** Free functions for TOML-based library persistence.

**load_library:**
```rust
// mirrors spec::load_gates pattern exactly
pub fn load_library(path: &Path) -> Result<CriteriaLibrary> {
    let content = std::fs::read_to_string(path).map_err(|source| AssayError::Io {
        operation: "reading criteria library".into(),
        path: path.to_path_buf(),
        source,
    })?;
    toml::from_str(&content).map_err(|e| AssayError::LibraryParse {
        path: path.to_path_buf(),
        message: crate::config::format_toml_error(&content, &e),
    })
}
```

**save_library (atomic):**
```rust
// mirrors save_session pattern (NamedTempFile → persist)
pub fn save_library(assay_dir: &Path, lib: &CriteriaLibrary) -> Result<PathBuf> {
    let criteria_dir = assay_dir.join("criteria");
    std::fs::create_dir_all(&criteria_dir)
        .map_err(|e| AssayError::io("creating criteria directory", &criteria_dir, e))?;
    validate_slug(&lib.name)?;
    let final_path = criteria_dir.join(format!("{}.toml", lib.name));
    let toml_str = toml::to_string_pretty(lib).map_err(...)?;
    let mut tmpfile = NamedTempFile::new_in(&criteria_dir)
        .map_err(|e| AssayError::io("creating temp file", &criteria_dir, e))?;
    tmpfile.write_all(toml_str.as_bytes())
        .map_err(|e| AssayError::io("writing library", &final_path, e))?;
    tmpfile.persist(&final_path)
        .map_err(|e| AssayError::io("persisting library", &final_path, e.error))?;
    Ok(final_path)
}
```

**scan_libraries:**
```rust
// mirrors spec::scan pattern
pub fn scan_libraries(assay_dir: &Path) -> Result<Vec<CriteriaLibrary>> {
    let criteria_dir = assay_dir.join("criteria");
    if !criteria_dir.is_dir() { return Ok(vec![]); }
    // read_dir, filter *.toml, load each, sort by name
}
```

### Pattern 2: resolve() — closure pair, pure function

**What:** Merges parent + library + own criteria into `ResolvedGate`.

**Merge order (own-wins):**
1. Parent criteria (from `extends`) — labeled `CriterionSource::Parent`
2. Library criteria (from each `include` slug, in order) — labeled `CriterionSource::Library { slug }`
3. Own criteria — labeled `CriterionSource::Own` — **wins** on name collision

**Algorithm sketch:**
```rust
pub fn resolve(
    gate: &GatesSpec,
    gate_slug: &str,
    load_gate: impl Fn(&str) -> Result<GatesSpec>,
    load_library: impl Fn(&str) -> Result<CriteriaLibrary>,
) -> Result<ResolvedGate> {
    // 1. Slug-validate extends and include values
    // 2. Cycle detection: if parent.extends == gate_slug → CycleDetected
    //    Also: if gate_slug == parent_slug → CycleDetected (self-extend)
    // 3. Load parent gate (single level), take its criteria as-is
    // 4. Load each library slug → CriterionSource::Library { slug }
    // 5. Build HashMap<name, ResolvedCriterion> in order (parent → library → own)
    //    Own overwrites silently.
    // 6. Return ResolvedGate with ordered Vec (insertion order: parent first, own last)
}
```

**Cycle detection for single-level:**
- Self-cycle: `gate_slug == extends_slug`
- Mutual cycle: load parent, check `parent.extends == Some(gate_slug)`
- Both cases → `AssayError::CycleDetected`

### Pattern 3: validate_slug()

```rust
pub fn validate_slug(value: &str) -> Result<()> {
    // Pattern: ^[a-z0-9][a-z0-9_-]*$  max 64 chars
    if value.is_empty() || value.len() > 64 {
        return Err(AssayError::InvalidSlug { slug: value.to_string(), reason: "..." });
    }
    let mut chars = value.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return Err(AssayError::InvalidSlug { ... });
    }
    if !value.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-') {
        return Err(AssayError::InvalidSlug { ... });
    }
    Ok(())
}
```

### Pattern 4: New assay-types structs (ResolvedGate, ResolvedCriterion, CriterionSource)

```rust
// Source: assay-types conventions (all existing types follow this pattern)

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CriterionSource {
    Own,
    Parent { gate_slug: String },
    Library { slug: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolvedCriterion {
    #[serde(flatten)]
    pub criterion: Criterion,
    pub source: CriterionSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolvedGate {
    pub gate_slug: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_slug: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub included_libraries: Vec<String>,
    pub criteria: Vec<ResolvedCriterion>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "resolved-gate",
        generate: || schemars::schema_for!(ResolvedGate),
    }
}
```

### Pattern 5: New AssayError variants

Following the exact `#[non_exhaustive]` + `thiserror::Error` pattern in `error.rs`:

```rust
/// Criteria library file parsing failed.
#[error("parsing criteria library `{path}`: {message}")]
LibraryParse { path: PathBuf, message: String },

/// Criteria library not found by slug.
#[error("criteria library `{slug}` not found in {criteria_dir}")]
LibraryNotFound { slug: String, criteria_dir: PathBuf },

/// Parent gate not found during resolution.
#[error("parent gate `{parent_slug}` not found (referenced from `{gate_slug}`)")]
ParentGateNotFound { gate_slug: String, parent_slug: String },

/// Circular extends chain detected.
#[error("circular extends detected: `{gate_slug}` and `{parent_slug}` extend each other")]
CycleDetected { gate_slug: String, parent_slug: String },

/// Invalid slug value.
#[error("invalid slug `{slug}`: {reason}")]
InvalidSlug { slug: String, reason: String },
```

### Anti-Patterns to Avoid

- **Trait objects for I/O abstraction:** The project uses closure pairs (`impl Fn`), not trait objects. Do not introduce a `GateLoader` trait.
- **Collecting all errors:** `resolve()` fails on first error. Do not return `Vec<AssayError>`.
- **Re-resolving parent's includes:** Parent criteria are taken as-is from the loaded `GatesSpec`. Do not recursively resolve parent's `include` list.
- **Mutable HashMap as output:** Return `Vec<ResolvedCriterion>` with defined order (parent → library → own), not an unordered map.
- **Direct file writes (no tempfile):** The CONTEXT.md marks atomicity as Claude's discretion, but `tempfile` is already a workspace dep and the pattern is proven. Use it for `save_library`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Fuzzy slug suggestions | Custom string diff | `spec::find_fuzzy_match` (already exists) | Levenshtein already implemented and tested |
| TOML parse error formatting | Custom formatter | `crate::config::format_toml_error` (already exists) | Produces caret-pointer output consistently |
| Atomic file writes | Manual tmp+rename | `tempfile::NamedTempFile` + `.persist()` | Race-free; already used in `save_session` and `history` |
| Directory scanning | Custom walker | `std::fs::read_dir` with existing filter pattern | Already done in `spec::scan` — direct adaptation |

**Key insight:** This codebase has already solved every file I/O, error formatting, and fuzzy-match problem this phase requires. The implementation task is adaptation, not invention.

## Common Pitfalls

### Pitfall 1: `#[serde(deny_unknown_fields)]` on `ResolvedCriterion` with `#[serde(flatten)]`

**What goes wrong:** `deny_unknown_fields` and `flatten` interact badly in serde — `flatten` pulls fields up from the inner struct, but `deny_unknown_fields` on the outer struct may reject them as unknown.

**Why it happens:** Serde's `deny_unknown_fields` + `flatten` combination is a known limitation.

**How to avoid:** Do NOT use `#[serde(deny_unknown_fields)]` on `ResolvedCriterion` if using `#[serde(flatten)]`. Only `Criterion` (the inner type) needs `deny_unknown_fields`. Alternatively, embed `Criterion` as a named field instead of flattening.

**Warning signs:** Compile-time serde errors mentioning "unknown field" during deserialization tests.

### Pitfall 2: `CriterionSource::Library` slug collision with criteria name collision

**What goes wrong:** Two libraries include a criterion with the same name. The merge semantics (own-wins) apply to library criteria too — but the order between libraries matters.

**Why it happens:** `include = ["lib-a", "lib-b"]` — if both define criterion `"tests-pass"`, which wins?

**How to avoid:** Establish clear merge order: parent → lib[0] → lib[1] → ... → own. Later entries in include list overwrite earlier ones; own criteria always win. Document this in the function's doc comment.

**Warning signs:** Flaky tests that depend on HashMap iteration order.

### Pitfall 3: Slug validation applied retroactively but not blocking existing load

**What goes wrong:** `validate_slug()` is applied to spec slugs and gate names retroactively. If existing `.assay/specs/` directories have slugs that fail the new pattern, `scan()` would break silently or noisily.

**Why it happens:** Retroactive validation on load is a breaking change for existing data.

**How to avoid:** Apply `validate_slug()` only in `resolve()` (on `extends`/`include` values) and in `save_library()`. Do NOT retrofit validation into `scan()` or `load_gates()` as part of this phase — that is a separate concern (SAFE-02, Phase 66).

**Warning signs:** Test failures on `spec::scan` for existing test fixtures with non-slug directory names.

### Pitfall 4: Self-extend detection requires comparing the gate's own slug

**What goes wrong:** `resolve()` receives `gate_slug` as a parameter but the `GatesSpec` struct has a `name` field. They may differ if the caller passes a mismatched slug.

**Why it happens:** `GatesSpec.name` is a display name, not necessarily the filesystem slug.

**How to avoid:** Cycle detection uses the `gate_slug` parameter (filesystem key), not `gate.name`. The `gate_slug` is the authoritative identity for extends-chain comparison.

### Pitfall 5: `tempfile::NamedTempFile::persist` error type

**What goes wrong:** `.persist(path)` returns `PersistError`, not `std::io::Error`. Calling `.map_err(|e| AssayError::io(..., e))` will fail to compile.

**Why it happens:** `PersistError` wraps the `NamedTempFile` back alongside the I/O error so you can recover it.

**How to avoid:** Use `.map_err(|e| AssayError::io("persisting library", &final_path, e.error))` — the `.error` field is the `std::io::Error`.

## Code Examples

### Existing: `save_session` atomic write pattern (assay-core/src/work_session.rs)
```rust
// Pattern to adapt for save_library
let mut tmpfile = NamedTempFile::new_in(&sessions_dir)
    .map_err(|e| AssayError::io("creating temp file for session", &sessions_dir, e))?;
tmpfile
    .write_all(json.as_bytes())
    .map_err(|e| AssayError::io("writing work session", &final_path, e))?;
tmpfile
    .persist(&final_path)
    .map_err(|e| AssayError::io("persisting work session", &final_path, e.error))?;
```

### Existing: `spec::scan` directory traversal pattern (assay-core/src/spec/mod.rs:683)
```rust
// Pattern to adapt for scan_libraries
let dir_entries = std::fs::read_dir(specs_dir).map_err(|source| AssayError::SpecScan {
    path: specs_dir.to_path_buf(),
    source,
})?;
let mut fs_entries: Vec<_> = dir_entries
    .filter_map(|entry| match entry {
        Ok(e) => Some(e),
        Err(source) => { /* push to errors */ None }
    })
    .collect();
fs_entries.sort_by_key(|e| e.path());
```

### Existing: `inventory::submit!` schema registration (assay-types/src/criteria_library.rs:41)
```rust
inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "criteria-library",
        generate: || schemars::schema_for!(CriteriaLibrary),
    }
}
```

### Existing: `find_fuzzy_match` usage pattern (assay-core/src/spec/mod.rs)
```rust
// For LibraryNotFound suggestions
let candidates: Vec<String> = existing_slugs.iter().cloned().collect();
let suggestion = find_fuzzy_match(slug, &candidates);
// Then embed suggestion in error message or variant
```

## State of the Art

| Old Approach | Current Approach | Notes |
|--------------|------------------|-------|
| Trait-based I/O abstraction | `impl Fn` closure pairs | Zero-trait convention established Phase 60+ |
| `validate_path_component` for all identifiers | `validate_slug()` for slug-typed values | `path_component` stays for non-slug use |
| No criteria libraries | `CriteriaLibrary` in assay-types (Phase 64) | Type exists; I/O functions are Phase 65 work |
| No gate inheritance resolution | `resolve()` in `spec::compose` | Phase 65 work |

## Open Questions

1. **`CriterionSource::Library` slug in `ResolvedCriterion`**
   - What we know: each criterion needs to carry which library it came from
   - What's unclear: CONTEXT.md grants discretion on annotation detail — simple enum is sufficient for Phase 65, but Phase 68 (MCP `spec_resolve`) may want the full override chain
   - Recommendation: use `CriterionSource::Library { slug: String }` (carries slug) — this is the minimal useful annotation. An override chain adds complexity with no consumer in Phase 65 or 68 scope.

2. **`save_library` TOML serialization format**
   - What we know: `toml::to_string` vs `toml::to_string_pretty` — project uses `to_string` for history/sessions (JSON pretty), but TOML files are human-authored
   - What's unclear: which TOML serializer function produces the best human-readable output
   - Recommendation: use `toml::to_string_pretty` for library files (human-authored) — mirrors how gate spec files are structured.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` (no external test runner) |
| Config file | none (cargo workspace) |
| Quick run command | `cargo test -p assay-core spec::compose` |
| Full suite command | `just test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| INHR-03 | Self-extend cycle detected | unit | `cargo test -p assay-core compose::tests::cycle_self_extend` | ❌ Wave 0 |
| INHR-03 | Mutual-extend cycle detected | unit | `cargo test -p assay-core compose::tests::cycle_mutual_extend` | ❌ Wave 0 |
| INHR-04 | ResolvedCriterion carries source=Own | unit | `cargo test -p assay-core compose::tests::source_annotation_own` | ❌ Wave 0 |
| INHR-04 | ResolvedCriterion carries source=Parent | unit | `cargo test -p assay-core compose::tests::source_annotation_parent` | ❌ Wave 0 |
| INHR-04 | Own criterion overwrites parent (own-wins) | unit | `cargo test -p assay-core compose::tests::own_wins_merge` | ❌ Wave 0 |
| CLIB-01 | CriteriaLibrary TOML roundtrip | unit | (already exists in assay-types) | ✅ |
| CLIB-01 | `load_library` parses valid file | unit | `cargo test -p assay-core compose::tests::load_library_valid` | ❌ Wave 0 |
| CLIB-02 | `resolve()` merges include library criteria | unit | `cargo test -p assay-core compose::tests::resolve_includes_library` | ❌ Wave 0 |
| CLIB-02 | `resolve()` rejects invalid include slug | unit | `cargo test -p assay-core compose::tests::resolve_invalid_include_slug` | ❌ Wave 0 |
| CLIB-03 | `save_library` writes valid TOML atomically | unit | `cargo test -p assay-core compose::tests::save_library_roundtrip` | ❌ Wave 0 |
| CLIB-03 | `scan_libraries` returns empty on missing dir | unit | `cargo test -p assay-core compose::tests::scan_libraries_missing_dir` | ❌ Wave 0 |
| CLIB-03 | `scan_libraries` returns all .toml files | unit | `cargo test -p assay-core compose::tests::scan_libraries_finds_files` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p assay-core spec::compose && cargo test -p assay-types criteria_library`
- **Per wave merge:** `just test`
- **Phase gate:** `just ready` (fmt-check + lint + test + deny) green before `/kata:verify-work`

### Wave 0 Gaps
- [ ] `crates/assay-core/src/spec/compose.rs` — new submodule with all tests inline
- [ ] `crates/assay-types/src/resolved_gate.rs` — new types + schema registration + unit tests
- [ ] No framework install needed — `#[test]` is built-in

## Sources

### Primary (HIGH confidence)
- Direct source inspection: `crates/assay-core/src/work_session.rs` — `save_session` atomic write pattern
- Direct source inspection: `crates/assay-core/src/spec/mod.rs:683` — `scan()` directory traversal pattern
- Direct source inspection: `crates/assay-core/src/history/mod.rs:27` — `validate_path_component` pattern
- Direct source inspection: `crates/assay-core/src/error.rs` — `AssayError` variant structure
- Direct source inspection: `crates/assay-types/src/criteria_library.rs` — `CriteriaLibrary` type + conventions
- Direct source inspection: `crates/assay-types/src/gates_spec.rs` — `GatesSpec.extends` and `.include` fields
- Direct source inspection: `crates/assay-core/src/gate/mod.rs:474` — `enriched_error_display` pattern
- Direct source inspection: `crates/assay-core/src/spec/mod.rs:44` — `find_fuzzy_match` function

### Secondary (MEDIUM confidence)
- Known serde limitation: `deny_unknown_fields` + `flatten` incompatibility is a documented serde behavior

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries are existing workspace deps; no research needed
- Architecture: HIGH — all patterns are direct copies/adaptations from existing production code
- Pitfalls: HIGH — serde flatten/deny_unknown_fields pitfall is well-known; others derived from direct code reading
- Validation architecture: HIGH — test framework and commands match existing workspace setup

**Research date:** 2026-04-11
**Valid until:** 2026-06-01 (stable domain, slow-moving Rust ecosystem)
