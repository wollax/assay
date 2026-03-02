# Phase 4: Schema Generation - Research

**Researched:** 2026-03-01
**Domain:** JSON Schema generation from Rust types via schemars 1.x
**Confidence:** HIGH

## Summary

Schema generation for Assay is straightforward. Schemars 1.x (already a workspace dependency at `"1"` with `chrono04` feature) generates Draft 2020-12 JSON Schema from `#[derive(JsonSchema)]` types with automatic `$schema`, `title`, and `description` metadata derived from doc comments. The output is deterministic because `serde_json` uses `BTreeMap` by default (alphabetically sorted keys), confirmed by repeated runs producing byte-identical output.

The main design question is "auto-discovery" — how the generator finds all schema-eligible types without a hardcoded list. The `inventory` crate (0.3.22, zero transitive deps) provides the cleanest decentralized registration pattern: each type registers itself at definition site, the generator iterates all registrations at runtime. For testing, `insta` (1.46.3) handles snapshot testing and `jsonschema` (0.43.0) handles roundtrip validation against Draft 2020-12 schemas.

**Primary recommendation:** Use `inventory` for type registration + an `assay-types/examples/generate-schemas.rs` binary that iterates registered types, writes one JSON file per root type to `schemas/`, and a `just schemas` recipe to run it.

## Standard Stack

The established libraries/tools for this domain:

### Core
| Library | Version | Purpose | Why Standard |
| --- | --- | --- | --- |
| schemars | 1.x (1.2.1 current) | JSON Schema generation from `#[derive(JsonSchema)]` types | Already in workspace, used by rmcp. Produces Draft 2020-12 by default |
| serde_json | 1.x | Serialize Schema to JSON string | Already in workspace. BTreeMap-backed `Value` guarantees deterministic key ordering |
| inventory | 0.3 | Distributed type registration for auto-discovery | Zero transitive deps, dtolnay-quality, linker-section-based collection |

### Supporting (dev-dependencies only)
| Library | Version | Purpose | When to Use |
| --- | --- | --- | --- |
| insta | 1.46 (feature: `json`) | Snapshot testing for schema determinism | Snapshot each generated schema; detect unintended drift |
| jsonschema | 0.43 | JSON Schema validation (Draft 2020-12) | Roundtrip validation: serialize known-good instance → validate against generated schema |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
| --- | --- | --- |
| inventory | linkme 0.3 | Similar distributed-slice pattern; slightly lower-level API but same effect. `inventory` has cleaner ergonomics for this use case |
| inventory | Declarative macro list in lib.rs | Technically still a "list" even if it serves dual purpose (re-export + registration). Violates the user's "no hardcoded list" requirement |
| inventory | Build script parsing lib.rs | Fragile, over-engineered, breaks on any formatting change |
| jsonschema | valico | valico only supports Draft 4/6/7, not Draft 2020-12 |

**Installation (workspace Cargo.toml additions):**
```toml
[workspace.dependencies]
inventory = "0.3"

# dev-dependencies (add to assay-types)
insta = { version = "1.46", features = ["json"] }
jsonschema = "0.43"
```

## Architecture Patterns

### Recommended Project Structure
```
crates/assay-types/
├── src/
│   ├── lib.rs              # pub mod + pub use + schema registry type
│   ├── schema_registry.rs  # SchemaEntry struct + inventory::collect!
│   ├── criterion.rs        # Types + inventory::submit! for each root type
│   └── gate.rs             # Types + inventory::submit! for each root type
├── examples/
│   └── generate-schemas.rs # Binary: iterate registry, write JSON files
├── tests/
│   ├── schema_roundtrip.rs # Validate instances against generated schemas
│   └── schema_snapshots.rs # Snapshot tests for determinism
schemas/
├── config.schema.json
├── criterion.schema.json
├── gate-kind.schema.json
├── gate-result.schema.json
├── gate.schema.json
├── review.schema.json
├── spec.schema.json
└── workflow.schema.json
```

### Pattern 1: Registry-Based Auto-Discovery

**What:** Each type that derives `JsonSchema` also registers a `SchemaEntry` via `inventory::submit!`. The generator binary iterates all entries.

**When to use:** Always — this is the core pattern for the phase.

**Example:**
```rust
// crates/assay-types/src/schema_registry.rs
use schemars::Schema;

/// A registered schema-generating type.
pub struct SchemaEntry {
    /// Kebab-case name for the output file (e.g., "gate-result")
    pub name: &'static str,
    /// Function that generates the root schema for this type
    pub generate: fn() -> Schema,
}

inventory::collect!(SchemaEntry);

/// Iterate all registered schema entries.
pub fn all_entries() -> impl Iterator<Item = &'static SchemaEntry> {
    inventory::iter::<SchemaEntry>
}
```

```rust
// In each type module (e.g., gate.rs), after the type definition:
inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "gate-result",
        generate: || schemars::schema_for!(GateResult),
    }
}
```

```rust
// crates/assay-types/examples/generate-schemas.rs
use assay_types::schema_registry;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("schemas");

    fs::create_dir_all(&out_dir).expect("create schemas dir");

    for entry in schema_registry::all_entries() {
        let schema = (entry.generate)();
        let json = serde_json::to_string_pretty(&schema)
            .expect("serialize schema");
        let path = out_dir.join(format!("{}.schema.json", entry.name));
        fs::write(&path, format!("{json}\n")).expect("write schema file");
        println!("  wrote {}", path.display());
    }
}
```

### Pattern 2: Post-Generation Metadata Enrichment

**What:** After schemars generates the schema, inject `$id` metadata for self-documenting schemas.

**When to use:** When schemas need to be standalone/self-documenting for external consumers.

**Example:**
```rust
// Source: schemars docs — Schema::insert() method
fn enrich_schema(schema: &mut schemars::Schema, name: &str) {
    // $schema is already added by schemars root_schema_for
    // $id for self-identification
    schema.insert(
        "$id".to_owned(),
        format!("https://assay.dev/schemas/{name}.schema.json").into(),
    );
}
```

Note: `$schema` (draft URI) and `title`/`description` (from doc comments) are already added automatically by schemars. Only `$id` needs manual injection.

### Pattern 3: Roundtrip Validation Test

**What:** Serialize a known-good instance to JSON, then validate it against the generated schema.

**When to use:** For every root type that has a schema — ensures the schema actually describes valid instances.

**Example:**
```rust
// crates/assay-types/tests/schema_roundtrip.rs
use assay_types::*;
use serde_json::json;

#[test]
fn config_instance_validates_against_schema() {
    let schema_json = schemars::schema_for!(Config);
    let schema_value = schema_json.to_value();
    let validator = jsonschema::draft202012::new(&schema_value)
        .expect("compile schema");

    let instance = Config {
        project_name: "test-project".into(),
        workflows: vec![Workflow {
            name: "ci".into(),
            specs: vec![],
            gates: vec![],
        }],
    };
    let instance_json = serde_json::to_value(&instance).unwrap();

    let result = validator.validate(&instance_json);
    assert!(result.is_ok(), "valid instance should pass: {result:?}");
}
```

### Pattern 4: Snapshot Test for Determinism

**What:** Snapshot each generated schema with `insta::assert_json_snapshot!` to detect unintended changes.

**When to use:** For every generated schema — ensures re-running produces identical output.

**Example:**
```rust
// crates/assay-types/tests/schema_snapshots.rs
use insta::assert_json_snapshot;

#[test]
fn config_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::Config);
    let value = schema.to_value();
    assert_json_snapshot!("config-schema", value);
}
```

### Anti-Patterns to Avoid
- **Hardcoded type list in the generator:** Violates the auto-discovery requirement. Every new type in Phase 5-6 would require touching the generator.
- **Generating schemas at build time (build.rs):** Build scripts run before compilation; they can't access the compiled types. Schema generation must be a post-build step.
- **Using `serde_json` with `preserve_order` feature:** Would change `BTreeMap` to `IndexMap`, making output order depend on insertion order rather than alphabetical sort. Would break determinism guarantee.
- **Generating one giant "all-types" schema file:** Individual schema files per root type are more useful for validation, IDE integration, and selective consumption.

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
| --- | --- | --- | --- |
| JSON Schema generation | Manual JSON construction | schemars `schema_for!` / `SchemaGenerator` | Serde attribute handling, `$ref` generation, `$defs` management, internal tagging support — all handled automatically |
| Draft 2020-12 compliance | Manual `$schema` injection | schemars default settings | Already produces `"$schema": "https://json-schema.org/draft/2020-12/schema"` by default |
| Schema metadata from docs | Manual `title`/`description` extraction | schemars doc comment integration | `///` comments automatically become `description`; first-line `#` headings become `title` |
| Type auto-discovery | Custom proc-macro or build script | `inventory` crate | Battle-tested linker-section approach, zero deps, used by pyo3/typetag/cucumber |
| Schema validation in tests | Manual JSON structure assertions | `jsonschema` crate | Full Draft 2020-12 support, `$ref` resolution, format validation |
| Snapshot testing | String comparison of JSON | `insta` with `json` feature | Pretty diff output, `cargo insta review` workflow, handles JSON formatting |

**Key insight:** Schemars 1.x already handles the hard parts (serde compatibility, `$ref`/`$defs`, internal tagging, chrono `DateTime` format). The generator binary is just plumbing: iterate types, call `schema_for!`, write files.

## Common Pitfalls

### Pitfall 1: Non-Deterministic Output from HashMap/IndexMap
**What goes wrong:** Schema files change on every generation despite no type changes.
**Why it happens:** Using `serde_json` with `preserve_order` feature or types that use `HashMap` internally.
**How to avoid:** Do NOT enable `preserve_order` on `serde_json`. Verify with `diff` that two consecutive runs produce identical output. The default `BTreeMap` backing guarantees alphabetical key ordering.
**Warning signs:** Schema files show up as changed in `git diff` after `just schemas` with no code changes.

### Pitfall 2: Forgetting to Register New Types
**What goes wrong:** A new type added in Phase 5-6 derives `JsonSchema` but doesn't get a schema file.
**Why it happens:** Developer adds the derive but forgets the `inventory::submit!` call.
**How to avoid:** Convention: the `inventory::submit!` block goes immediately after the type definition. Document this in CONTRIBUTING.md. A CI check that runs `just schemas` and verifies no new types are missing could catch this, but is likely overkill for v0.1.
**Warning signs:** Type has `#[derive(JsonSchema)]` but no corresponding `.schema.json` file.

### Pitfall 3: Schema `$ref` Paths Break in Standalone Files
**What goes wrong:** A `Config` schema references `"$ref": "#/$defs/Workflow"` — the `Workflow` definition is embedded inside the `Config` schema. If someone expects `Workflow` to be in its own file, the `$ref` won't resolve.
**Why it happens:** Schemars bundles all `$defs` into the root schema. Each root schema is self-contained.
**How to avoid:** Accept this behavior — it's correct for standalone schema files. Each file validates independently. Types that appear in `$defs` of a parent schema do NOT need their own separate schema file unless they are independently useful (like `Criterion` or `GateKind`).
**Warning signs:** External tools failing to resolve `$ref` — usually means the tool doesn't support JSON Schema `$defs` properly.

### Pitfall 4: `skip_serializing_if` vs. Schema `required`
**What goes wrong:** Fields with `#[serde(skip_serializing_if = "Option::is_none")]` still appear in the schema, but are correctly NOT in `required`. However, the schema type becomes `["string", "null"]` rather than just `"string"`.
**Why it happens:** Schemars reflects the Rust type (`Option<String>`) accurately. The `skip_serializing_if` only affects serialization, not the type's schema.
**How to avoid:** This is correct behavior. The schema accurately describes what values are valid for deserialization. Don't try to "fix" this.
**Warning signs:** Schema showing `"type": ["string", "null"]` for `Option` fields — this is correct.

### Pitfall 5: Trailing Newline Inconsistency
**What goes wrong:** Schema files don't end with a newline, or some do and some don't, causing noisy diffs.
**Why it happens:** `serde_json::to_string_pretty` does NOT append a trailing newline.
**How to avoid:** Always append `\n` after the JSON when writing to file: `format!("{json}\n")`.
**Warning signs:** `git diff` showing "no newline at end of file" warnings.

## Code Examples

Verified patterns from official sources:

### Generating a Root Schema (schemars 1.x)
```rust
// Source: https://docs.rs/schemars/latest/schemars/macro.schema_for.html
// schema_for! expands to: SchemaGenerator::default().into_root_schema_for::<T>()
let schema = schemars::schema_for!(MyType);

// Automatically includes:
//   "$schema": "https://json-schema.org/draft/2020-12/schema"
//   "title": "MyType" (from type name)
//   "description": "..." (from /// doc comments)
//   "$defs": { ... } (for referenced types)
```

### Inserting $id Metadata
```rust
// Source: https://docs.rs/schemars/latest/schemars/struct.Schema.html
let mut schema = schemars::schema_for!(Config);
schema.insert("$id".to_owned(), "https://assay.dev/schemas/config.schema.json".into());
```

### Validating Against Draft 2020-12 Schema
```rust
// Source: https://docs.rs/jsonschema/latest/jsonschema/
let schema_value = schema.to_value(); // schemars::Schema → serde_json::Value
let validator = jsonschema::draft202012::new(&schema_value)
    .expect("valid schema");

assert!(validator.is_valid(&instance_json));

// For detailed errors:
for error in validator.iter_errors(&instance_json) {
    eprintln!("Validation error: {} at {}", error, error.instance_path);
}
```

### Snapshot Testing with Insta
```rust
// Source: https://docs.rs/insta/latest/insta/
// Requires: insta = { version = "1.46", features = ["json"] }
use insta::assert_json_snapshot;

#[test]
fn my_type_schema_is_stable() {
    let schema = schemars::schema_for!(MyType);
    assert_json_snapshot!("my-type-schema", schema.to_value());
}
// Snapshots stored in tests/snapshots/my_type_schema_is_stable@my-type-schema.snap
```

### inventory Registration Pattern
```rust
// Source: https://docs.rs/inventory/latest/inventory/
// Define the registry type
pub struct SchemaEntry {
    pub name: &'static str,
    pub generate: fn() -> schemars::Schema,
}
inventory::collect!(SchemaEntry);

// Register from any module in the crate
inventory::submit! {
    SchemaEntry {
        name: "config",
        generate: || schemars::schema_for!(Config),
    }
}

// Iterate all registered entries
for entry in inventory::iter::<SchemaEntry> {
    let schema = (entry.generate)();
    // ... write to file
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
| --- | --- | --- | --- |
| schemars 0.8 (`RootSchema` struct) | schemars 1.x (`Schema` wrapping `serde_json::Value`) | schemars 1.0.0 | Simpler API, `json_schema!` macro, `Transform` trait replaces `Visitor` |
| JSON Schema Draft 7 default | JSON Schema Draft 2020-12 default | schemars 1.0.0 | Modern schema draft, `$defs` instead of `definitions` |
| `schemars::schema::SchemaObject` field access | `Schema::insert()`/`Schema::get()` map-style access | schemars 1.0.0 | No more nested struct navigation; direct key-value manipulation |
| `visit::Visitor` trait | `transform::Transform` trait | schemars 1.0.0 | Simpler, can use closures instead of trait impls |

**Deprecated/outdated:**
- `schemars::schema` module: Gone in 1.x. `Schema` is now at `schemars::Schema`.
- `RootSchema` type: Gone. All methods return `Schema` now.
- `SchemaObject`: Gone. Use `Schema` with map-style access.
- `definitions` path: Default is now `$defs` (JSON Schema 2020-12), not `definitions` (Draft 7).

## Open Questions

Things that couldn't be fully resolved:

1. **Which types should get individual schema files?**
   - What we know: `Config` is the clear top-level type. `GateResult`, `Criterion`, `GateKind` are independently useful for validation.
   - What's unclear: Whether `Spec`, `Gate`, `Review`, `Workflow` need individual files when they're always embedded in `Config`'s `$defs`.
   - Recommendation: Generate individual files for ALL public types that derive `JsonSchema`. It's cheap (one file per type), and having too many schemas is better than missing one an external tool needs. The `inventory` pattern makes this automatic anyway — every registered type gets a file.

2. **$id URI scheme**
   - What we know: `$id` should be a URI. It does NOT need to be resolvable. URNs are valid per spec.
   - What's unclear: Whether to use `https://assay.dev/schemas/...` (aspirational URL) or a URN scheme.
   - Recommendation: Use `https://assay.dev/schemas/{name}.schema.json` — it's conventional, human-readable, and can become resolvable later if schemas are published. The domain doesn't need to exist today.

3. **Git tracking of generated schemas**
   - What we know: `schemas/` is already in git (from Phase 1 scaffold). Generated files are deterministic.
   - What's unclear: Whether to commit generated schemas or add `schemas/*.json` to `.gitignore`.
   - Recommendation: Commit them. Benefits: consumers can reference schemas without building the project; CI can verify schemas are up-to-date via `just schemas-check`; IDE/editor JSON validation works out-of-the-box from the repo.

4. **`just schemas` overwrite vs. check behavior**
   - What we know: Need both modes — overwrite for development, check for CI.
   - Recommendation: Two recipes: `just schemas` (overwrite, for development) and `just schemas-check` (generate to temp dir, diff against committed, fail if different — for CI/`just ready`).

## Sources

### Primary (HIGH confidence)
- schemars docs.rs — API reference for `Schema`, `SchemaGenerator`, `SchemaSettings`, `schema_for!` macro (https://docs.rs/schemars/latest/schemars/)
- schemars GitHub source — Verified `schema_for!` expands to `SchemaGenerator::default().into_root_schema_for::<T>()`, confirmed `$schema` meta-schema is auto-inserted by `root_schema_for` (https://github.com/GREsau/schemars)
- schemars migration guide — 0.8 → 1.x breaking changes documented (https://graham.cool/schemars/migrating/)
- schemars attributes — Doc comment integration, `#[schemars(title, description)]` (https://graham.cool/schemars/deriving/attributes/)
- jsonschema docs.rs — Validation API, `draft202012::new()`, `is_valid()`, `iter_errors()` (https://docs.rs/jsonschema/latest/jsonschema/)
- inventory docs.rs — `collect!`, `submit!`, `iter` API (https://docs.rs/inventory/latest/inventory/)
- insta docs.rs — `assert_json_snapshot!`, `json` feature flag (https://docs.rs/insta/latest/insta/)
- **Direct verification** — Built and ran schemars 1.x against actual `assay-types` crate. Confirmed: Draft 2020-12 `$schema` auto-added, `title` from type name, `description` from doc comments, `serde(tag = "kind")` produces correct `oneOf` with `const` discriminator, deterministic output across runs, `skip_serializing_if` fields correctly omitted from `required`.

### Secondary (MEDIUM confidence)
- serde_json determinism — Default `BTreeMap` backing (not `IndexMap`) ensures alphabetical key ordering. Verified via `cargo tree` that `preserve_order` is not enabled in the workspace. (https://github.com/serde-rs/json/issues/54)
- JSON Schema `$id` practices — URIs don't need to be resolvable; URL-style `$id` is conventional (https://github.com/orgs/json-schema-org/discussions/205)

### Tertiary (LOW confidence)
- None — all findings verified with primary or secondary sources.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — schemars already in workspace, API verified against real types, output format confirmed
- Architecture: HIGH — inventory pattern well-documented and verified; generator binary approach specified in roadmap
- Pitfalls: HIGH — determinism verified empirically; serde attribute behavior confirmed with real output

**Research date:** 2026-03-01
**Valid until:** 2026-06-01 (stable domain; schemars 1.x is mature, inventory 0.3 is stable)
