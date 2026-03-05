# Phase 15: Run History CLI — Research

## Standard Stack

| Concern | Solution | Already in workspace? |
|---------|----------|-----------------------|
| CLI argument parsing | `clap` 4 (derive) | Yes |
| Date/time handling | `chrono` 0.4 | Yes |
| Serialization | `serde` / `serde_json` | Yes |
| Config parsing | `toml` 0.8 | Yes |
| Temp files (tests) | `tempfile` 3 | Yes |

No new dependencies needed. All relative-time formatting is hand-rolled (see Architecture Patterns below). Confidence: **HIGH**.

## Architecture Patterns

### 1. CLI Command Structure (clap derive)

Add a `History` variant to the existing `GateCommand` enum. Use positional args with `Option` for the optional run-id.

```rust
#[derive(Subcommand)]
enum GateCommand {
    Run { /* existing */ },
    /// View gate run history for a spec
    History {
        /// Spec name
        name: String,
        /// Optional run ID to show detail view
        run_id: Option<String>,
        /// Show most recent run in detail
        #[arg(long)]
        last: bool,
        /// Maximum number of entries to display
        #[arg(long, default_value = "20")]
        limit: usize,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}
```

clap handles positional `Option<String>` cleanly — the second positional is consumed only when present. `--last` and `run_id` should be `conflicts_with` each other. Confidence: **HIGH**.

### 2. Relative Timestamp Formatting

chrono provides `DateTime::signed_duration_since()` returning a `TimeDelta`. Use `TimeDelta::num_seconds()` and branch on thresholds. No external "timeago" crate needed.

```rust
fn format_relative_timestamp(ts: &DateTime<Utc>, now: &DateTime<Utc>) -> String {
    let delta = now.signed_duration_since(*ts);
    let secs = delta.num_seconds();
    if secs < 0 { return ts.format("%Y-%m-%d %H:%M").to_string(); }
    match secs {
        0..=59 => format!("{}s", secs),
        60..=3599 => format!("{}m", secs / 60),
        3600..=86399 => format!("{}h", secs / 3600),
        _ => ts.format("%Y-%m-%d %H:%M").to_string(),
    }
}
```

The CONTEXT specifies: compact relative for < 1 day (`2m`, `4h`), absolute beyond 1 day (`2026-03-05 14:30`). Confidence: **HIGH**.

### 3. Table Rendering

Follow the existing `print_criteria_table()` pattern in `main.rs`: manual column-width calculation, `format!` with padding, `println!` for stdout, Unicode box-drawing for separators. No table rendering crate.

Column widths: `#` (index), Timestamp (fixed ~16 chars), Status (6 chars), Passed/Failed/Skipped/ReqFailed/AdvFailed (numeric, 3-4 chars each), Duration (variable).

Use the same `colors_enabled()` / `NO_COLOR` check. Apply green for pass, red for fail status. Confidence: **HIGH**.

### 4. Pruning Integration into `save()`

The `save()` function in `history/mod.rs` currently writes atomically and returns the path. Pruning should happen **after** the successful persist (never delete old files if the new write failed).

```rust
pub fn save(assay_dir: &Path, record: &GateRunRecord, max_history: usize) -> Result<SaveResult> {
    // ... existing atomic write ...
    let pruned = if max_history > 0 {
        prune(assay_dir, &record.summary.spec_name, max_history)?
    } else {
        Vec::new()
    };
    Ok(SaveResult { path: final_path, pruned })
}
```

The `prune()` function: call `list()` to get sorted run IDs, if `len > max_history`, remove the oldest `len - max_history` files. Return the list of pruned IDs for the caller to optionally log.

**Breaking change consideration:** Adding `max_history` parameter to `save()` changes its signature. This affects the MCP server and CLI callers. Alternative: make `save()` accept an `Option<usize>` or a config struct. The `Option<usize>` approach (`None` = no pruning, `Some(0)` = unlimited, `Some(n)` = prune to n) is simplest. Confidence: **HIGH**.

### 5. Config Extension

Add `max_history` to `GatesConfig` in `assay-types/src/lib.rs`:

```rust
pub struct GatesConfig {
    #[serde(default = "default_timeout")]
    pub default_timeout: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
    #[serde(default = "default_max_history")]
    pub max_history: usize,
}

fn default_max_history() -> usize { 1000 }
```

`deny_unknown_fields` is already on `GatesConfig`, so the new field needs `serde(default)` to avoid breaking existing config files that omit it. Confidence: **HIGH**.

### 6. Detail View (single run)

When `run_id` is provided or `--last` is used, load the full `GateRunRecord` via `history::load()` and render a formatted summary. For `--json`, just `serde_json::to_string_pretty()` the record.

For human-readable detail: show header (run ID, timestamp, version, working dir), then a per-criterion table similar to `gate run` streaming output but static. Reuse `format_pass()`/`format_fail()` helpers. Confidence: **HIGH**.

### 7. Resolving the assay_dir

The existing `gate run` handler uses `project_root()` to find cwd, then joins `.assay`. The history commands need the same `root.join(".assay")` as the `assay_dir` parameter. Follow the `load_gate_context()` pattern. Confidence: **HIGH**.

## Don't Hand-Roll

| Problem | Use Instead |
|---------|-------------|
| Table rendering library | Stick with manual `format!` + padding (matches existing codebase) |
| Relative time crate (e.g., `timeago`) | Hand-roll with `chrono::TimeDelta` (3 branches, trivial) |
| File pruning / rotation crate | Hand-roll: `list()` + `std::fs::remove_file()` on oldest entries |

The codebase convention is zero unnecessary dependencies. All three of these are trivially implemented inline.

## Common Pitfalls

### P1: `save()` signature change breaks callers (HIGH confidence)
Adding `max_history` to `save()` breaks the existing call sites. The MCP server and any tests that call `save()` directly must be updated. Use `Option<usize>` to keep the change backwards-compatible in spirit (`None` = no pruning, preserving old behavior).

### P2: Pruning races with concurrent saves (MEDIUM confidence)
If two `gate run` processes save simultaneously, both might read the list before either prunes, leading to one extra file. This is benign (eventual consistency on next save). Do NOT add locking — the cost exceeds the risk.

### P3: `--last` with empty history (HIGH confidence)
`--last` on a spec with no history must print "No history for <spec>" and exit 0, not panic on empty `list()`. Same for `--limit 0`.

### P4: `deny_unknown_fields` on `GatesConfig` + new field (HIGH confidence)
Existing `.assay/config.toml` files lack `max_history`. The field MUST have `#[serde(default = "...")]` or deserialization will fail on projects that haven't added it.

### P5: Duration formatting (MEDIUM confidence)
`total_duration_ms` is `u64` in milliseconds. Display as `1.5s`, `42ms`, `2m 15s` etc. Match existing patterns — the codebase currently doesn't format durations for display, so define a simple helper.

### P6: Table alignment with ANSI codes (HIGH confidence)
The existing codebase accounts for `ANSI_COLOR_OVERHEAD` (9 bytes) when calculating column widths with color. The history table must do the same or columns will misalign when colors are enabled.

### P7: Pruning stderr suppression under `--json` (HIGH confidence)
The CONTEXT specifies: suppress pruning messages when `--json` is active. The pruning happens in `save()` (core layer) but JSON mode is a CLI concern. Solution: `save()` returns pruned IDs, CLI decides whether to log them.

## Code Examples

### Relative timestamp (complete implementation)

```rust
use chrono::{DateTime, Utc};

fn format_timestamp(ts: &DateTime<Utc>, now: &DateTime<Utc>) -> String {
    let secs = now.signed_duration_since(*ts).num_seconds();
    if secs < 0 {
        return ts.format("%Y-%m-%d %H:%M").to_string();
    }
    match secs {
        0..=59 => format!("{secs}s"),
        60..=3599 => format!("{}m", secs / 60),
        3600..=86399 => format!("{}h", secs / 3600),
        _ => ts.format("%Y-%m-%d %H:%M").to_string(),
    }
}
```

### Prune function (core layer)

```rust
pub fn prune(assay_dir: &Path, spec_name: &str, max_history: usize) -> Result<Vec<String>> {
    let ids = list(assay_dir, spec_name)?;
    if ids.len() <= max_history {
        return Ok(Vec::new());
    }
    let to_remove = ids.len() - max_history;
    let pruned: Vec<String> = ids.into_iter().take(to_remove).collect();
    let results_dir = assay_dir.join("results").join(spec_name);
    for id in &pruned {
        let path = results_dir.join(format!("{id}.json"));
        std::fs::remove_file(&path).map_err(|source| AssayError::Io {
            operation: "pruning old run record".into(),
            path,
            source,
        })?;
    }
    Ok(pruned)
}
```

### Duration formatting helper

```rust
fn format_duration_ms(ms: u64) -> String {
    if ms < 1000 {
        format!("{ms}ms")
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        let mins = ms / 60_000;
        let secs = (ms % 60_000) / 1000;
        format!("{mins}m {secs}s")
    }
}
```

### Save with pruning (updated signature)

```rust
pub struct SaveResult {
    pub path: PathBuf,
    pub pruned: Vec<String>,
}

pub fn save(
    assay_dir: &Path,
    record: &GateRunRecord,
    max_history: Option<usize>,
) -> Result<SaveResult> {
    // ... existing atomic write logic ...
    let pruned = match max_history {
        Some(0) | None => Vec::new(),  // 0 = unlimited, None = no pruning
        Some(limit) => prune(assay_dir, &record.summary.spec_name, limit)?,
    };
    Ok(SaveResult { path: final_path, pruned })
}
```

## Key API Surface (Existing Code Reference)

| Function | Location | Signature |
|----------|----------|-----------|
| `history::save()` | `assay-core/src/history/mod.rs:72` | `(assay_dir: &Path, record: &GateRunRecord) -> Result<PathBuf>` |
| `history::load()` | `assay-core/src/history/mod.rs:128` | `(assay_dir: &Path, spec_name: &str, run_id: &str) -> Result<GateRunRecord>` |
| `history::list()` | `assay-core/src/history/mod.rs:153` | `(assay_dir: &Path, spec_name: &str) -> Result<Vec<String>>` |
| `history::generate_run_id()` | `assay-core/src/history/mod.rs:47` | `(timestamp: &DateTime<Utc>) -> String` |
| `config::load()` | `assay-core/src/config/mod.rs:79` | `(root: &Path) -> Result<Config>` |
| `load_gate_context()` | `assay-cli/src/main.rs:582` | `() -> (PathBuf, Config, PathBuf, Option<u64>)` |
| `colors_enabled()` | `assay-cli/src/main.rs:191` | `() -> bool` |
| `format_pass()` / `format_fail()` | `assay-cli/src/main.rs:213,218` | `(color: bool) -> &'static str` |

## Task Breakdown Guidance

Natural task boundaries for the planner:

1. **Config extension** — Add `max_history` to `GatesConfig`, update validation, add tests
2. **Prune function** — Add `prune()` to history module, integrate into `save()`, update `SaveResult`, update all callers
3. **History table command** — Add `History` variant to `GateCommand`, implement table rendering with relative timestamps
4. **Detail view** — Implement single-run detail view (`run_id` and `--last` modes), including `--json` output
5. **Integration tests** — End-to-end CLI tests for all command forms, edge cases (empty history, pruning behavior)
