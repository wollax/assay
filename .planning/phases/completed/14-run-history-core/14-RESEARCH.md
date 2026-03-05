# Phase 14: Run History Core - Research

**Researched:** 2026-03-04
**Confidence:** HIGH (all findings verified against codebase and official docs)

---

## Standard Stack

| Concern | Solution | Status | Confidence |
|---------|----------|--------|------------|
| Atomic writes | `tempfile` crate (`NamedTempFile::new_in` + `persist`) | Already in workspace deps (`tempfile = "3"`); dev-dep in assay-core, needs promotion to regular dep | HIGH |
| JSON serialization | `serde_json` crate | Already in workspace deps (`serde_json = "1"`); **not in assay-core deps at all** — needs adding as regular dep | HIGH |
| Timestamps | `chrono` crate (`DateTime<Utc>`, `Utc::now()`, `format!`) | Already used throughout project (`chrono = "0.4"` with `serde` feature) | HIGH |
| Random hex suffix | `std::collections::hash_map::RandomState` | No new crate needed per STATE.md decision | HIGH |
| Directory creation | `std::fs::create_dir_all` | stdlib, no dependency | HIGH |
| Type schemas | `schemars` + `inventory` | Existing pattern in assay-types | HIGH |

### Dependency changes required for assay-core

```toml
# Cargo.toml [dependencies] additions:
serde_json.workspace = true   # NEW — currently not a dep at all
tempfile.workspace = true      # PROMOTE from [dev-dependencies] to [dependencies]
```

### Dependency changes required for assay-types

```toml
# No changes — GateRunRecord goes in assay-types, which already has serde, schemars, chrono, inventory
```

---

## Architecture Patterns

### Module placement

- **`GateRunRecord` type** → `assay-types/src/gate_run.rs` (alongside existing `GateRunSummary` and `CriterionResult`)
- **History save/load/list logic** → `assay-core/src/history/mod.rs` (new module)
- **Re-export** → `assay-core/src/lib.rs` adds `pub mod history;`

This follows the established pattern: serializable types in `assay-types`, business logic in `assay-core`.

### GateRunRecord type design

**Recommendation: Wrap GateRunSummary, don't duplicate.**

`GateRunRecord` should be a wrapper struct containing `GateRunSummary` as a field, plus metadata. This avoids duplicating all fields and lets existing consumers that work with `GateRunSummary` continue unchanged.

```rust
// In assay-types/src/gate_run.rs
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GateRunRecord {
    /// Unique run identifier (timestamp + random suffix, e.g. "20260304T223015Z-a3f1b2")
    pub run_id: String,
    /// Version of assay that produced this record (for schema migration)
    pub assay_version: String,
    /// When this run was initiated
    pub timestamp: DateTime<Utc>,
    /// The complete gate evaluation results
    pub summary: GateRunSummary,
}
```

**Key decisions embedded:**
- `spec_name` is already inside `GateRunSummary.spec_name` — no need to duplicate at record level. Downstream consumers deserialize the full record and access `record.summary.spec_name`.
- `deny_unknown_fields` per CONTEXT.md decision — records are versioned artifacts.
- `run_id` stored in the record for self-describing records even if moved/copied outside the directory structure.
- `assay_version` comes from `env!("CARGO_PKG_VERSION")` at save time.

### Embed spec name in record (recommended)

The spec name is already in `GateRunSummary.spec_name`. Since `GateRunRecord` wraps `GateRunSummary`, the spec name is automatically embedded. This is important for downstream consumers (MCP, CLI) that may load records without directory context.

### File naming convention

Per CONTEXT.md: ISO-ish compact timestamps like `20260304T223015Z.json`.

With the 6-char random hex suffix from STATE.md: `20260304T223015Z-a3f1b2.json`

Format string: `"%Y%m%dT%H%M%SZ"` for the timestamp part, then `-{hex_suffix}`.

```rust
fn generate_filename(timestamp: &DateTime<Utc>) -> String {
    let ts = timestamp.format("%Y%m%dT%H%M%SZ");
    let suffix = random_hex_suffix();
    format!("{ts}-{suffix}.json")
}
```

### Random hex without new crate

Use `std::collections::hash_map::RandomState` which provides per-instance random seeds via the OS-seeded SipHash:

```rust
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};

fn random_hex_suffix() -> String {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u8(0); // finalize
    format!("{:06x}", hasher.finish() & 0x00FF_FFFF)
}
```

This gives 24 bits of entropy (16M possible values) — sufficient for uniqueness within a 1-second window. Combined with second-resolution timestamps, collisions are astronomically unlikely.

### Directory layout

```
.assay/
  results/
    my-spec/
      20260304T223015Z-a3f1b2.json
      20260304T223115Z-c7d2e9.json
    another-spec/
      20260305T101030Z-1a2b3c.json
```

Already gitignored via `.assay/.gitignore` which contains `results/`.

### Spec name slugification

The codebase already uses slugs for spec entries (`SpecEntry::slug()`). The slug is derived from directory names or filename stems in `spec::scan()`. For the results directory, use the same slug.

**Sanitization approach:** Replace characters that are unsafe in filenames with hyphens, collapse consecutive hyphens, trim leading/trailing hyphens.

```rust
fn slugify(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect::<String>()
        .replace("--", "-")
        .trim_matches('-')
        .to_lowercase()
}
```

However, since spec names come from user-authored TOML files and the existing `SpecEntry::slug()` already derives a filesystem-safe slug, the history module should accept a `&str` slug parameter (not raw spec name) and let callers provide the already-sanitized slug. This avoids duplicate sanitization logic.

---

## Don't Hand-Roll

| Problem | Use Instead | Why |
|---------|------------|-----|
| Atomic file writes | `tempfile::NamedTempFile::new_in()` + `.persist()` | Handles temp file cleanup on failure, cross-platform rename semantics |
| Random bytes | `std::collections::hash_map::RandomState` | No `rand` crate needed per STATE.md decision |
| Timestamp formatting | `chrono::DateTime::format()` | Already used throughout the project |
| JSON serialization | `serde_json::to_string_pretty()` | Consistent with CLI output pattern |
| Directory creation | `std::fs::create_dir_all()` | Handles nested creation, idempotent |
| Version string | `env!("CARGO_PKG_VERSION")` | Compile-time, no runtime cost |

---

## Common Pitfalls

### 1. Temp file on different filesystem (CRITICAL)

`NamedTempFile::new()` creates temp files in the OS temp directory (e.g. `/tmp`). On many systems, `/tmp` is a different filesystem from the project directory. `rename()` across filesystems fails with `EXDEV` (cross-device link).

**Fix:** Always use `NamedTempFile::new_in(target_dir)` to create the temp file in the same directory as the final destination. This guarantees same-filesystem rename.

### 2. Missing `sync_all()` before persist

Without `sync_all()`, the file contents may be in OS buffers when `persist()` renames. A power failure after rename but before flush would leave a valid filename pointing to incomplete/empty content.

**Fix:** Call `tmpfile.as_file().sync_all()?` before `tmpfile.persist(target)`.

### 3. `deny_unknown_fields` breaks forward compatibility

If a newer version of assay adds fields to `GateRunRecord`, an older version cannot deserialize those records. This is the **intended behavior** per CONTEXT.md — mismatches should be caught immediately. But this means the `assay_version` field is important for error messages.

**Mitigation:** Include `assay_version` in the record. When deserialization fails, surface the version mismatch in error messages.

### 4. Slug collision with different spec names

Two different spec names could theoretically slugify to the same directory name. This is low-risk because spec names are already validated for uniqueness in the spec scanning phase, and slugs derive from filesystem entries which are unique.

**Mitigation:** Accept slugs from callers rather than slugifying internally. The spec scanning code already handles uniqueness.

### 5. evaluate_all returns GateRunSummary, not GateRunRecord

The existing `evaluate_all()` and `evaluate_all_gates()` functions return `GateRunSummary`. The history module needs to wrap this into `GateRunRecord` with metadata.

**Recommendation:** Don't modify `evaluate_all()`. Let callers construct `GateRunRecord` from the returned summary, then call `history::save()`. This keeps evaluation and persistence decoupled, improving testability.

```rust
// Caller pattern (CLI or MCP):
let summary = gate::evaluate_all(&spec, &working_dir, cli_timeout, config_timeout);
let record = GateRunRecord::new(summary); // adds run_id, assay_version, timestamp
history::save(&results_dir, &slug, &record)?;
```

### 6. Pretty-print vs compact JSON

The codebase uses `to_string_pretty` for all user-facing JSON. For history files, pretty-printing is appropriate — these are audit artifacts that users may inspect manually.

---

## Code Examples

### Atomic save pattern

```rust
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

pub fn save(results_dir: &Path, slug: &str, record: &GateRunRecord) -> Result<PathBuf> {
    let spec_dir = results_dir.join(slug);
    std::fs::create_dir_all(&spec_dir).map_err(|source| AssayError::Io {
        operation: "creating results directory".into(),
        path: spec_dir.clone(),
        source,
    })?;

    let filename = generate_filename(&record.timestamp);
    let target = spec_dir.join(&filename);

    let json = serde_json::to_string_pretty(record).map_err(|e| AssayError::Io {
        operation: "serializing gate run record".into(),
        path: target.clone(),
        source: std::io::Error::new(std::io::ErrorKind::Other, e),
    })?;

    // Create temp file in same directory for atomic rename
    let mut tmpfile = NamedTempFile::new_in(&spec_dir).map_err(|source| AssayError::Io {
        operation: "creating temp file for atomic write".into(),
        path: spec_dir.clone(),
        source,
    })?;

    tmpfile.write_all(json.as_bytes()).map_err(|source| AssayError::Io {
        operation: "writing gate run record".into(),
        path: target.clone(),
        source,
    })?;

    tmpfile.as_file().sync_all().map_err(|source| AssayError::Io {
        operation: "syncing gate run record".into(),
        path: target.clone(),
        source,
    })?;

    tmpfile.persist(&target).map_err(|e| AssayError::Io {
        operation: "persisting gate run record".into(),
        path: target.clone(),
        source: e.error,
    })?;

    Ok(target)
}
```

### GateRunRecord constructor

```rust
impl GateRunRecord {
    pub fn new(summary: GateRunSummary) -> Self {
        let timestamp = Utc::now();
        let suffix = random_hex_suffix();
        let run_id = format!("{}-{suffix}", timestamp.format("%Y%m%dT%H%M%SZ"));

        Self {
            run_id,
            assay_version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp,
            summary,
        }
    }
}
```

Note: `env!("CARGO_PKG_VERSION")` will resolve to the `assay-types` version since `GateRunRecord` lives there. Since all workspace crates share `version.workspace = true`, this is the correct project version.

### Load a single record

```rust
pub fn load(path: &Path) -> Result<GateRunRecord> {
    let content = std::fs::read_to_string(path).map_err(|source| AssayError::Io {
        operation: "reading gate run record".into(),
        path: path.to_path_buf(),
        source,
    })?;

    serde_json::from_str(&content).map_err(|e| AssayError::Io {
        operation: "deserializing gate run record".into(),
        path: path.to_path_buf(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
    })
}
```

### List records for a spec (sorted by filename = chronological)

```rust
pub fn list(results_dir: &Path, slug: &str) -> Result<Vec<PathBuf>> {
    let spec_dir = results_dir.join(slug);
    if !spec_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries: Vec<PathBuf> = std::fs::read_dir(&spec_dir)
        .map_err(|source| AssayError::Io {
            operation: "listing run history".into(),
            path: spec_dir.clone(),
            source,
        })?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();

    entries.sort(); // Filenames are timestamp-prefixed, so lexicographic = chronological
    Ok(entries)
}
```

### Testing concurrent saves

```rust
#[test]
fn concurrent_saves_produce_distinct_files() {
    let dir = tempfile::tempdir().unwrap();
    let results_dir = dir.path().join("results");

    let summary = make_test_summary("test-spec");

    // Save two records "concurrently" (same second)
    let record1 = GateRunRecord::new(summary.clone());
    let record2 = GateRunRecord::new(summary.clone());

    let path1 = history::save(&results_dir, "test-spec", &record1).unwrap();
    let path2 = history::save(&results_dir, "test-spec", &record2).unwrap();

    // Different files (random suffix ensures uniqueness)
    assert_ne!(path1, path2);

    // Both deserialize correctly
    let loaded1 = history::load(&path1).unwrap();
    let loaded2 = history::load(&path2).unwrap();
    assert_eq!(loaded1.run_id, record1.run_id);
    assert_eq!(loaded2.run_id, record2.run_id);
}
```

### Testing crash resilience (no corrupt files)

```rust
#[test]
fn partial_write_leaves_no_corrupt_file() {
    let dir = tempfile::tempdir().unwrap();
    let spec_dir = dir.path().join("results").join("test-spec");
    std::fs::create_dir_all(&spec_dir).unwrap();

    // Verify that if we drop a NamedTempFile without persisting,
    // no file remains in the spec directory
    let tmpfile = NamedTempFile::new_in(&spec_dir).unwrap();
    let _tmp_path = tmpfile.path().to_path_buf();
    drop(tmpfile); // simulates crash — temp file is cleaned up

    // Only .json files in the directory
    let entries: Vec<_> = std::fs::read_dir(&spec_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(entries.is_empty(), "No files should remain after dropping temp file");
}
```

### Testing roundtrip serialization

```rust
#[test]
fn gate_run_record_json_roundtrip() {
    let record = GateRunRecord::new(make_test_summary("roundtrip-spec"));
    let json = serde_json::to_string_pretty(&record).unwrap();
    let deserialized: GateRunRecord = serde_json::from_str(&json).unwrap();

    assert_eq!(record.run_id, deserialized.run_id);
    assert_eq!(record.assay_version, deserialized.assay_version);
    assert_eq!(record.summary.spec_name, deserialized.summary.spec_name);
    assert_eq!(record.summary.passed, deserialized.summary.passed);
    assert_eq!(record.summary.failed, deserialized.summary.failed);
}
```

---

## Discretion Recommendations

| Decision Area | Recommendation | Rationale |
|---------------|---------------|-----------|
| Environment metadata | Include `working_dir` only (as `Option<String>`) | Useful for debugging; hostname and git ref are over-engineering for v0.2 |
| Spec name location | Embedded via `GateRunSummary.spec_name` (already there) | Self-describing records; no duplication needed |
| Record wrapping | `GateRunRecord` wraps `GateRunSummary` via `summary` field | Cleanest serde, avoids field duplication, existing consumers unchanged |
| Auto-save vs explicit | Callers save explicitly | Keeps evaluation pure/testable; CLI and MCP both already have the integration point |
| Save failure behavior | Return `Result` — let callers decide | CLI can warn and continue; MCP can fail the tool call. Phase 15/17 decide policy |
| Uniqueness strategy | Timestamp + 6-char random hex suffix | Per STATE.md decision; 24-bit entropy in the suffix |
| Temp file location | Same directory as target (`NamedTempFile::new_in`) | Required for atomic rename (same filesystem) |
| File granularity | One file per `evaluate_all()` call | Matches "one file per gate evaluation call" from CONTEXT.md; current model evaluates all criteria in one call |
| Pretty-print JSON | Yes (`to_string_pretty`) | Consistent with CLI output; these are audit files users inspect |

---

## New Error Variants Needed

The `AssayError` enum will need no new variants — the existing `Io` variant with its `operation`/`path`/`source` fields covers all history I/O operations. The code examples above demonstrate this pattern.

If deserialization of corrupt/incompatible records needs a distinct error, consider adding a variant in a follow-up, but for Phase 14 the `Io` variant with `InvalidData` kind suffices.

---

## Integration Points for Downstream Phases

| Phase | What it needs from Phase 14 |
|-------|---------------------------|
| Phase 15 (CLI history) | `history::list()` to enumerate records, `history::load()` to read them |
| Phase 17 (MCP history) | Same `list()` and `load()` functions, called from async context via `spawn_blocking` |
| Phase 15 (retention) | `history::list()` returns sorted paths; retention deletes oldest files beyond count limit |

The API surface (`save`, `load`, `list`) is intentionally minimal and covers all downstream needs.

---

*Phase: 14-run-history-core*
*Research completed: 2026-03-04*
