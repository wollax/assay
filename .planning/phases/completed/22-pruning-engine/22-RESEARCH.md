# Phase 22: Pruning Engine - Research

**Researched:** 2026-03-06
**Domain:** Composable JSONL processing pipeline in Rust, session file manipulation
**Confidence:** HIGH

## Summary

The pruning engine is a composable pipeline of pure functions operating on parsed session JSONL entries. Each strategy receives `Vec<ParsedEntry>` and returns a filtered/modified `Vec<ParsedEntry>`, with line-deletion strategies running before content-modification strategies.

The existing codebase already provides the critical foundation: `ParsedEntry` (parser.rs), `SessionEntry` typed enum (context.rs), `BloatCategory` categorization (diagnostics.rs), and team message extraction logic (checkpoint/extractor.rs). The pruning engine extends these with a strategy trait, pipeline executor, backup/restore module, and CLI surface under `assay context prune`.

**Primary recommendation:** Work at the `ParsedEntry` level using the existing parser, but add raw line preservation to `ParsedEntry` for lossless round-tripping. Strategies are standalone functions (not trait objects) that compose via sequential application.

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|-------------|
| serde_json | 1 (already in workspace) | JSON parsing, Value manipulation, re-serialization | Already used throughout, Value type enables in-place content modification |
| regex-lite | 0.1 (already in workspace) | Pattern matching for system reminders, metadata patterns | Already used in diagnostics.rs for `<system-reminder>` detection |
| chrono | 0.4 (already in workspace) | Timestamp generation for backup filenames | Already used in checkpoint persistence |
| tempfile | 3 (already in workspace) | Atomic file writes via temp + rename | Already in workspace, standard for safe file replacement |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|------------|
| dirs | 6 (already in workspace) | Home directory resolution for `.assay/backups/` | Backup directory creation |

### No New Dependencies Needed

All required libraries are already in the workspace `Cargo.toml`. No new dependencies need to be added.

## Architecture Patterns

### Recommended Module Structure

```
crates/assay-core/src/context/
  mod.rs              # Add: pub mod pruning;
  pruning/
    mod.rs            # Pipeline executor, PruneResult, re-exports
    strategy.rs       # Strategy enum, strategy dispatch, ordering
    strategies/
      mod.rs
      progress_collapse.rs
      stale_reads.rs
      tool_output_trim.rs
      thinking_blocks.rs
      metadata_strip.rs
      system_reminder_dedup.rs
    protection.rs     # Team message protection (is_protected check)
    backup.rs         # Backup/restore operations
    report.rs         # Dry-run report formatting, PruneSummary types

crates/assay-types/src/
  context.rs          # Add: PruneReport, PruneSummary, PruneStrategy enum

crates/assay-cli/src/
  main.rs             # Add: ContextCommand::Prune variant
```

### Pattern 1: Strategy as Pure Function

**What:** Each strategy is a standalone function with signature `fn(entries: Vec<ParsedEntry>, tier: PrescriptionTier) -> StrategyResult`. No trait objects, no dynamic dispatch. The `StrategyResult` wraps the modified entries plus per-strategy statistics.

**When to use:** Always. This is the locked decision from CONTEXT.md.

**Example:**
```rust
/// Result of applying a single pruning strategy.
pub struct StrategyResult {
    /// The entries after this strategy has been applied.
    pub entries: Vec<ParsedEntry>,
    /// Number of lines removed by this strategy.
    pub lines_removed: usize,
    /// Number of lines modified (content trimmed) by this strategy.
    pub lines_modified: usize,
    /// Bytes saved by this strategy.
    pub bytes_saved: u64,
    /// Number of protected lines skipped.
    pub protected_skipped: usize,
    /// Sample removals for dry-run display (up to 3).
    pub samples: Vec<PruneSample>,
}

/// A sample of what was pruned, for dry-run display.
pub struct PruneSample {
    pub line_number: usize,
    pub description: String,
    pub bytes: u64,
}

/// Execute the progress-collapse strategy.
pub fn progress_collapse(
    entries: Vec<ParsedEntry>,
    _tier: PrescriptionTier,
    protected: &HashSet<usize>,
) -> StrategyResult {
    let mut result_entries = Vec::with_capacity(entries.len());
    let mut lines_removed = 0;
    let mut bytes_saved = 0u64;
    let mut protected_skipped = 0;
    let mut samples = Vec::new();

    for entry in entries {
        if protected.contains(&entry.line_number) {
            result_entries.push(entry);
            protected_skipped += 1;
            continue;
        }
        if matches!(entry.entry, SessionEntry::Progress(_)) {
            bytes_saved += entry.raw_bytes as u64;
            lines_removed += 1;
            if samples.len() < 3 {
                samples.push(PruneSample {
                    line_number: entry.line_number,
                    description: "Progress tick".into(),
                    bytes: entry.raw_bytes as u64,
                });
            }
        } else {
            result_entries.push(entry);
        }
    }

    StrategyResult {
        entries: result_entries,
        lines_removed,
        lines_modified: 0,
        bytes_saved,
        protected_skipped,
        samples,
    }
}
```

### Pattern 2: Pipeline Executor

**What:** The pipeline executor applies strategies sequentially, collecting per-strategy results.

**Example:**
```rust
pub struct PruneResult {
    pub entries: Vec<ParsedEntry>,
    pub strategy_results: Vec<(PruneStrategy, StrategyResult)>,
    pub original_size: u64,
    pub final_size: u64,
}

pub fn execute_pipeline(
    entries: Vec<ParsedEntry>,
    strategies: &[PruneStrategy],
    tier: PrescriptionTier,
    protected_lines: &HashSet<usize>,
) -> PruneResult {
    let original_count = entries.len();
    let original_size: u64 = entries.iter().map(|e| e.raw_bytes as u64).sum();
    let mut current = entries;
    let mut strategy_results = Vec::new();

    for strategy in strategies {
        let result = strategy.apply(current, tier, protected_lines);
        current = result.entries.clone(); // or use a take pattern
        strategy_results.push((*strategy, result));
    }

    let final_size: u64 = current.iter().map(|e| e.raw_bytes as u64).sum();

    PruneResult {
        entries: current,
        strategy_results,
        original_size,
        final_size,
    }
}
```

### Pattern 3: Protection Set (Pre-computed)

**What:** Before the pipeline runs, scan all entries once to build a `HashSet<usize>` of protected line numbers. Each strategy receives this set and skips protected lines.

**Why:** Avoids redundant pattern matching per-strategy. The protection check is O(1) per line.

**Example:**
```rust
/// Tool names that indicate team coordination messages.
const PROTECTED_TOOL_NAMES: &[&str] = &[
    "TaskCreate", "TaskUpdate", "TaskOutput", "TaskGet", "TaskList", "TaskStop",
    "TeamCreate", "TeamDelete", "SendMessage",
    "Task",  // Catch-all for Task tool
];

pub fn build_protection_set(entries: &[ParsedEntry]) -> HashSet<usize> {
    let mut protected = HashSet::new();
    for entry in entries {
        if is_team_message(entry) {
            protected.insert(entry.line_number);
        }
    }
    protected
}

fn is_team_message(entry: &ParsedEntry) -> bool {
    // Check user entries for tool_use blocks with protected names
    // Check progress entries for nested content blocks with protected names
    // (Reuse patterns from checkpoint/extractor.rs)
    match &entry.entry {
        SessionEntry::User(u) => {
            if let Some(msg) = &u.message {
                if let Some(blocks) = msg.as_array() {
                    return blocks.iter().any(|b| {
                        b.get("name")
                            .and_then(|n| n.as_str())
                            .is_some_and(|n| PROTECTED_TOOL_NAMES.contains(&n))
                    });
                }
            }
            false
        }
        SessionEntry::Progress(p) => {
            if let Some(data) = &p.data {
                if let Some(blocks) = data
                    .pointer("/message/message/content")
                    .and_then(|c| c.as_array())
                {
                    return blocks.iter().any(|b| {
                        b.get("name")
                            .and_then(|n| n.as_str())
                            .is_some_and(|n| PROTECTED_TOOL_NAMES.contains(&n))
                    });
                }
            }
            false
        }
        _ => false,
    }
}
```

### Pattern 4: Raw Line Preservation for Lossless Round-Trip

**What:** Extend `ParsedEntry` to optionally store the raw JSON line string, enabling lossless write-back for unmodified entries. Modified entries are re-serialized.

**Why critical:** The `SessionEntry` type uses `#[serde(flatten)]` for `EntryMetadata` and `serde_json::Value` for variable fields. Round-tripping through serde may reorder fields, change whitespace, or lose fields not captured by the struct. For unmodified lines, the original bytes must be preserved exactly.

**Example:**
```rust
// Extended ParsedEntry (modify existing struct)
pub struct ParsedEntry {
    pub entry: SessionEntry,
    pub line_number: usize,
    pub raw_bytes: usize,
    /// The original JSON line, preserved for lossless write-back.
    pub raw_line: String,
}
```

This is a non-breaking change to `ParsedEntry` since it's only used internally in `assay-core`. The parser already has the line available; it just needs to clone/store it.

### Pattern 5: Atomic File Write

**What:** Write the pruned session to a temporary file in the same directory, then rename atomically. This prevents data loss if the process crashes mid-write.

**Example:**
```rust
use std::io::Write;
use tempfile::NamedTempFile;

pub fn write_session(entries: &[ParsedEntry], target: &Path) -> crate::Result<()> {
    let dir = target.parent().unwrap_or(Path::new("."));
    let mut tmp = NamedTempFile::new_in(dir).map_err(/* ... */)?;

    for entry in entries {
        writeln!(tmp, "{}", entry.raw_line).map_err(/* ... */)?;
    }

    tmp.persist(target).map_err(/* ... */)?;
    Ok(())
}
```

### Anti-Patterns to Avoid

- **Trait objects for strategies:** Over-engineering for 6 concrete strategies. Use an enum + match dispatch instead.
- **In-place file mutation:** Never modify the JSONL file in place. Always write to temp + rename.
- **Re-parsing per strategy:** Parse once, pass `Vec<ParsedEntry>` through the pipeline. Never re-read from disk between strategies.
- **Interactive confirmation:** CONTEXT.md explicitly decided against this. `--execute` is sufficient intent.
- **Partial writes on error:** If any strategy fails, abort the entire pipeline. Don't write a partially-pruned file.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|------------|-------------|-----|
| Atomic file writes | Custom temp+rename logic | `tempfile::NamedTempFile::persist()` | Handles cross-device rename, cleanup on drop |
| JSON round-trip preservation | Custom serializer | Store `raw_line: String` in ParsedEntry | serde round-trip may reorder fields due to `#[serde(flatten)]` |
| System reminder detection | Custom string scanning | `regex_lite::Regex` (already used in diagnostics.rs) | Regex already proven in the codebase |
| Team message detection | New extraction logic | Reuse patterns from `checkpoint/extractor.rs` | Same tool name matching logic |
| Backup timestamping | Manual formatting | `chrono::Utc::now().format("%Y%m%dT%H%M%S")` | Already in workspace |
| File path manipulation | String concatenation | `std::path::PathBuf` joining | Cross-platform correctness |

**Key insight:** The diagnostics module (`categorize_bloat`) already implements detection logic for all 6 bloat categories. The pruning strategies mirror these categories 1:1. Extract shared detection predicates rather than duplicating the matching logic.

## Common Pitfalls

### Pitfall 1: Serde Flatten Round-Trip Data Loss

**What goes wrong:** `SessionEntry` uses `#[serde(flatten)]` for `EntryMetadata`. Serializing a deserialized entry may reorder JSON fields, change numeric formatting, or lose fields not captured in the struct.
**Why it happens:** `flatten` collects unknown fields into a virtual map during deserialization, but field ordering is not guaranteed.
**How to avoid:** Store the raw line in `ParsedEntry`. For unmodified entries, write the raw line verbatim. Only re-serialize entries that were actually modified by a strategy.
**Warning signs:** Diff of original vs pruned file shows changes in lines that should have been untouched.

### Pitfall 2: Content Modification Without Size Update

**What goes wrong:** A strategy modifies content within an entry (e.g., trims tool output) but doesn't update `raw_bytes` or `raw_line`, causing incorrect savings calculations.
**Why it happens:** The bytes/size tracking is based on `raw_bytes`, which reflects the original line.
**How to avoid:** When a strategy modifies content, it must re-serialize the entry to a new `raw_line` and update `raw_bytes` accordingly. Provide a helper: `ParsedEntry::update_content(&mut self, new_entry: SessionEntry)`.
**Warning signs:** Reported savings don't match actual file size reduction.

### Pitfall 3: Protected Line Numbers Drift After Line Deletion

**What goes wrong:** Protection set uses original line numbers, but after a strategy deletes lines, remaining entries still have their original `line_number` values. This is actually fine because `line_number` is a stable identifier from the original file, not an index.
**Why it happens:** Misunderstanding that `line_number` needs updating.
**How to avoid:** Use `line_number` as a stable identifier throughout the pipeline. Never re-index line numbers between strategies. The `HashSet<usize>` protection set remains valid because it references original line numbers.
**Warning signs:** Protected lines being pruned, or wrong lines being protected.

### Pitfall 4: Strategy Ordering Matters for Accuracy

**What goes wrong:** Running content-trimming strategies before line-deletion strategies leads to wasted work (trimming content that will be deleted) and inaccurate per-strategy savings.
**Why it happens:** Strategies are independent functions but their composition order affects reported savings.
**How to avoid:** CONTEXT.md locks the ordering: line-deletion strategies first (progress-collapse, stale-reads), then content-modification strategies (metadata-strip, thinking-blocks, tool-output-trim, system-reminder-dedup). Prescription tiers encode this ordering.
**Warning signs:** Strategies report savings for content that was removed by a later strategy.

### Pitfall 5: Backup Directory Size Growth

**What goes wrong:** Session JSONL files can be 10-50MB. Without retention limits, backups consume significant disk space.
**Why it happens:** Each `--execute` creates a backup. Frequent pruning of large sessions compounds.
**How to avoid:** Implement retention limit (configurable, default 5). Prune oldest backups when limit is exceeded. Delete backups per-session, not globally.
**Warning signs:** `.assay/backups/` grows unboundedly.

### Pitfall 6: Stale Read Detection Edge Cases

**What goes wrong:** Stale read detection must match by file path, but paths may differ by trailing slash, relative vs absolute, or symlink resolution.
**Why it happens:** Tool inputs contain user-specified paths that may not be canonicalized.
**How to avoid:** Normalize paths for comparison (strip trailing slashes, no need for full canonicalization since these are the exact strings from the JSONL). The existing diagnostics code uses exact string matching, which is correct for this use case since the paths come from Claude's tool calls which are consistent.
**Warning signs:** Duplicate reads not detected, or unrelated files incorrectly matched.

## Code Examples

### Strategy Enum and Dispatch

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PruneStrategy {
    ProgressCollapse,
    SystemReminderDedup,
    MetadataStrip,
    StaleReads,
    ThinkingBlocks,
    ToolOutputTrim,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PrescriptionTier {
    Gentle,
    Standard,
    Aggressive,
}

impl PrescriptionTier {
    /// Returns the strategies for this tier in execution order.
    /// Line-deletion strategies come first, then content-modification.
    pub fn strategies(&self) -> &[PruneStrategy] {
        match self {
            Self::Gentle => &[
                PruneStrategy::ProgressCollapse,
                PruneStrategy::SystemReminderDedup,
            ],
            Self::Standard => &[
                PruneStrategy::ProgressCollapse,
                PruneStrategy::StaleReads,
                PruneStrategy::SystemReminderDedup,
                PruneStrategy::MetadataStrip,
            ],
            Self::Aggressive => &[
                PruneStrategy::ProgressCollapse,
                PruneStrategy::StaleReads,
                PruneStrategy::ThinkingBlocks,
                PruneStrategy::ToolOutputTrim,
                PruneStrategy::SystemReminderDedup,
                PruneStrategy::MetadataStrip,
            ],
        }
    }
}

impl PruneStrategy {
    pub fn apply(
        &self,
        entries: Vec<ParsedEntry>,
        tier: PrescriptionTier,
        protected: &HashSet<usize>,
    ) -> StrategyResult {
        match self {
            Self::ProgressCollapse => strategies::progress_collapse(entries, tier, protected),
            Self::SystemReminderDedup => strategies::system_reminder_dedup(entries, tier, protected),
            Self::MetadataStrip => strategies::metadata_strip(entries, tier, protected),
            Self::StaleReads => strategies::stale_reads(entries, tier, protected),
            Self::ThinkingBlocks => strategies::thinking_blocks(entries, tier, protected),
            Self::ToolOutputTrim => strategies::tool_output_trim(entries, tier, protected),
        }
    }
}
```

### Backup and Restore

```rust
use std::path::{Path, PathBuf};

const DEFAULT_RETENTION_LIMIT: usize = 5;

pub fn backup_session(
    session_path: &Path,
    backup_dir: &Path,
) -> crate::Result<PathBuf> {
    std::fs::create_dir_all(backup_dir).map_err(/* ... */)?;

    let session_name = session_path.file_stem().unwrap_or_default().to_string_lossy();
    let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    let backup_name = format!("{session_name}_{timestamp}.jsonl");
    let backup_path = backup_dir.join(&backup_name);

    std::fs::copy(session_path, &backup_path).map_err(/* ... */)?;

    // Enforce retention limit
    prune_old_backups(backup_dir, &session_name, DEFAULT_RETENTION_LIMIT)?;

    Ok(backup_path)
}

pub fn list_backups(
    backup_dir: &Path,
    session_id: &str,
) -> crate::Result<Vec<PathBuf>> {
    // List files matching "{session_id}_*.jsonl", sorted newest first
    todo!()
}

pub fn restore_backup(
    backup_path: &Path,
    session_path: &Path,
) -> crate::Result<()> {
    std::fs::copy(backup_path, session_path).map_err(/* ... */)?;
    Ok(())
}
```

### CLI Integration

```rust
// In ContextCommand enum:
/// Prune session bloat using composable strategies
Prune {
    /// Session ID (required)
    session_id: String,

    /// Prescription tier: gentle, standard, aggressive
    #[arg(long, default_value = "standard")]
    tier: String,

    /// Run individual strategy instead of prescription
    #[arg(long, conflicts_with = "tier")]
    strategy: Option<String>,

    /// Actually modify the session file (default is dry-run)
    #[arg(long)]
    execute: bool,

    /// Restore a previous backup
    #[arg(long, conflicts_with_all = ["tier", "strategy", "execute"])]
    restore: bool,

    /// Output as JSON
    #[arg(long)]
    json: bool,
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|-------------|-----------------|--------------|--------|
| Python JSONL manipulation (Cozempic) | Rust native with typed parsing | This project | 10-100x performance, type safety |
| Re-serialize all lines | Raw line preservation + selective re-serialization | Current best practice | Lossless round-trip for unmodified lines |
| Trait object strategies | Enum dispatch with standalone functions | Rust idiom | Zero overhead, exhaustive matching |
| In-place file mutation | Atomic temp + rename | Standard practice | Crash-safe writes |

**Deprecated/outdated:**
- `serde_json::RawValue` was considered for zero-copy pass-through but adds complexity without sufficient benefit. Storing `raw_line: String` in `ParsedEntry` is simpler and achieves the same lossless round-trip goal since each line is processed independently.

## Open Questions

1. **Content-modification strategies: how to update `raw_line`?**
   - What we know: When a strategy modifies an entry's content (e.g., strips thinking blocks), the `raw_line` must be regenerated via `serde_json::to_string(&entry)`.
   - What's unclear: Whether serde round-trip of modified entries preserves enough fidelity for Claude Code to re-read the session correctly. The `#[serde(flatten)]` on `EntryMetadata` may reorder fields.
   - Recommendation: Test with real session files. Claude Code's JSONL parser is likely field-order-independent (standard JSON). Flag this for integration testing.

2. **Tool-output-trim heuristic thresholds**
   - What we know: CONTEXT.md says "initial: simpler line-based heuristics with extension point." Cozempic uses 8KB / 100 lines.
   - What's unclear: Optimal thresholds for Assay's use cases.
   - Recommendation: Start with configurable defaults (e.g., keep first 20 + last 20 lines for outputs > 100 lines). Mark as tunable.

## Sources

### Primary (HIGH confidence)
- Existing codebase: `crates/assay-core/src/context/parser.rs`, `diagnostics.rs`, `tokens.rs`
- Existing codebase: `crates/assay-types/src/context.rs` — all types verified by reading source
- Existing codebase: `crates/assay-core/src/checkpoint/extractor.rs` — team message detection patterns
- serde_json docs (docs.rs) — RawValue, Value manipulation
- Cozempic README (github.com/Ruya-AI/cozempic) — reference implementation strategies

### Secondary (MEDIUM confidence)
- serde_json `#[serde(flatten)]` round-trip behavior — verified via docs, but edge cases exist

### Tertiary (LOW confidence)
- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - all libraries already in workspace, no new deps needed
- Architecture: HIGH - patterns derived directly from existing codebase structure and CONTEXT.md decisions
- Pitfalls: HIGH - identified from codebase analysis (flatten round-trip, raw line preservation)
- Code examples: HIGH - based on existing code patterns in the same codebase

**Research date:** 2026-03-06
**Valid until:** 2026-04-06 (stable domain, no external dependencies changing)
