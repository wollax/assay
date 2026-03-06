# Phase 20: Session JSONL Parser & Token Diagnostics - Research

**Researched:** 2026-03-06
**Domain:** Claude Code session file parsing, token diagnostics, CLI/MCP integration
**Confidence:** HIGH

## Summary

Claude Code stores session data as JSONL files under `~/.claude/projects/<project-slug>/<session-uuid>.jsonl`. Each line is a JSON object with a `type` field discriminating between `user`, `assistant`, `progress`, `system`, `file-history-snapshot`, `queue-operation`, and `pr-link` entry types. Token usage data lives in `message.usage` on `assistant`-type entries and includes `input_tokens`, `output_tokens`, `cache_creation_input_tokens`, and `cache_read_input_tokens`. Progress entries (hook_progress, agent_progress, bash_progress) dominate session files by count (77% in a representative 830-entry session).

The project slug format is the absolute path with `/` replaced by `-` (e.g., `/Users/wollax/Git/personal/assay` becomes `-Users-wollax-Git-personal-assay`). Session discovery maps the current working directory to a project slug, then finds JSONL files in that directory. The `~/.claude/history.jsonl` file contains entries with `project` (absolute path), `sessionId`, `timestamp`, and `display` fields -- this serves as the project-to-session index.

**Primary recommendation:** Build the parser as a new `context` module in `assay-core` with types in `assay-types`. Use `serde_json::from_str` line-by-line (not `StreamDeserializer`) since JSONL entries are newline-delimited. Parse into a typed enum (`SessionEntry`) with `#[serde(tag = "type")]` discrimination. For the `estimate_tokens` fast path, read only the last 50KB of the file and parse backwards for the last `assistant` entry with `usage`.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| serde + serde_json | 1.x | JSONL parsing/serialization | Already in workspace, zero new deps |
| chrono | 0.4 | Timestamp parsing | Already in workspace |
| clap | 4.x | CLI subcommand (`context diagnose`, `context list`) | Already in workspace |
| rmcp | 0.17 | MCP tool handlers (`context_diagnose`, `estimate_tokens`) | Already in workspace |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| schemars | 1.x | JSON Schema for MCP param/response types | Already in workspace; use for MCP tool params |
| dirs | 6.x | Platform-correct home directory (~) | **NEW** -- more robust than `env::var("HOME")` for cross-platform |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Line-by-line serde_json | serde_json StreamDeserializer | StreamDeserializer is for concatenated JSON, not newline-delimited. Line-by-line is simpler and allows per-line error handling. |
| dirs crate | `std::env::var("HOME")` | Works on macOS/Linux but not Windows. dirs is minimal and correct. |
| memmap2 for large files | BufReader line-by-line | mmap adds complexity; BufReader is sufficient since session files are typically <10MB. The quick_token_estimate tail-read pattern uses seek + read which is already fast. |
| tiktoken-rs for heuristic estimation | chars-per-token ratio (3.7 default) | tiktoken-rs adds a significant dependency. Character heuristic is what Cozempic uses and is accurate enough for diagnostics. |

**Installation:**
```bash
# Only new workspace dependency needed:
cargo add dirs@6 --package assay-core
# Add to workspace Cargo.toml: dirs = "6"
```

## Architecture Patterns

### Recommended Project Structure
```
crates/assay-types/src/
  context.rs               # SessionEntry enum, UsageData, BloatCategory, DiagnosticsReport, etc.

crates/assay-core/src/
  context/
    mod.rs                 # Public API: diagnose(), list_sessions(), estimate_tokens()
    parser.rs              # JSONL line-by-line parser, SessionEntry deserialization
    discovery.rs           # Session file discovery: project slug, find sessions, resolve session
    diagnostics.rs         # Bloat categorization, token counting, context window %
    tokens.rs              # Token extraction (exact from usage), heuristic estimation

crates/assay-cli/src/main.rs
  (add Context subcommand with Diagnose + List)

crates/assay-mcp/src/server.rs
  (add context_diagnose + estimate_tokens tool handlers)
```

### Pattern 1: Tagged Enum Deserialization for JSONL Entries
**What:** Use `#[serde(tag = "type")]` to discriminate session entry types
**When to use:** Parsing each JSONL line into a typed Rust enum
**Example:**
```rust
// In assay-types/src/context.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum SessionEntry {
    User(UserEntry),
    Assistant(AssistantEntry),
    Progress(ProgressEntry),
    System(SystemEntry),
    #[serde(rename = "file-history-snapshot")]
    FileHistorySnapshot(FileHistorySnapshotEntry),
    #[serde(rename = "queue-operation")]
    QueueOperation(serde_json::Value),  // Not needed for diagnostics
    #[serde(rename = "pr-link")]
    PrLink(serde_json::Value),          // Not needed for diagnostics
}
```

### Pattern 2: Two-Phase Token Counting (Exact + Heuristic)
**What:** Prefer exact `usage` fields from assistant messages; fall back to character-based estimation
**When to use:** Always -- some entries lack usage data (user messages, tool results)
**Example:**
```rust
// Total context = input_tokens + cache_creation_input_tokens + cache_read_input_tokens
// This represents what's in the context window for the LAST assistant turn
pub fn extract_usage_tokens(entries: &[ParsedEntry]) -> Option<UsageData> {
    entries.iter().rev()
        .filter(|e| matches!(&e.entry, SessionEntry::Assistant(_)))
        .filter(|e| !e.is_sidechain)
        .find_map(|e| e.usage())
}
```

### Pattern 3: Tail-Read for Fast Token Estimation
**What:** Read only the last N bytes of a file, parse backwards for usage data
**When to use:** `estimate_tokens` MCP tool performance target
**Example:**
```rust
pub fn quick_token_estimate(path: &Path) -> io::Result<Option<u64>> {
    let file = File::open(path)?;
    let file_size = file.metadata()?.len();
    let read_size = file_size.min(50 * 1024); // 50KB tail
    let mut buf = vec![0u8; read_size as usize];
    file.seek(SeekFrom::End(-(read_size as i64)))?;
    file.read_exact(&mut buf)?;
    // Skip first partial line, parse backwards for assistant with usage
    // ...
}
```

### Pattern 4: Project Slug Convention
**What:** Claude Code maps absolute paths to project slugs by replacing `/` with `-`
**When to use:** Session discovery -- converting CWD to find the right project directory
**Verified from:** Live `~/.claude/projects/` directory structure and Cozempic `session.py`
```rust
pub fn cwd_to_project_slug(cwd: &Path) -> String {
    cwd.to_string_lossy().replace('/', "-")
}
// /Users/wollax/Git/personal/assay -> -Users-wollax-Git-personal-assay
```

### Anti-Patterns to Avoid
- **Deserializing entire JSONL into strongly-typed structs for all fields:** Session entries have many optional/variable fields. Use `serde_json::Value` for fields not needed for diagnostics (e.g., `content` block details beyond type discrimination). Only strongly type what you need.
- **Loading entire file into memory as a single string:** Use `BufReader::lines()` for streaming line-by-line processing. Session files can be multi-MB.
- **Treating `isSidechain` entries as main context:** Sidechain messages (subagent conversations) do NOT count toward the main context window. Always filter them out for token calculations.
- **Counting thinking blocks as context tokens:** Thinking blocks are ephemeral and not included in the context window. Cozempic explicitly excludes them from token heuristic estimation.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Home directory detection | `env::var("HOME")` | `dirs::home_dir()` | Platform-correct, handles edge cases |
| JSON line parsing | Custom tokenizer | `serde_json::from_str` per line | Battle-tested, handles all JSON edge cases |
| Context window sizes per model | Hardcoded single value | Lookup table (model -> window size) | Different models have different limits; Cozempic maintains a map |
| Bloat byte size estimation | Custom text length counting | `serde_json::to_string().len()` for blocks | Consistent with actual serialized size |
| UUID generation for session IDs | Custom format | Existing UUID pattern from session files | Session IDs are UUIDs, already standard |

**Key insight:** The JSONL format is Claude Code's internal format -- it is NOT a public API. The schema may change between Claude Code versions. Design defensively: unknown `type` values should be captured as `Unknown(serde_json::Value)` and gracefully skipped rather than causing parse failures.

## Common Pitfalls

### Pitfall 1: Assuming All Entries Have `message` Field
**What goes wrong:** Not all JSONL entry types have a `message` field. `progress`, `file-history-snapshot`, `queue-operation`, and `pr-link` entries use different structures.
**Why it happens:** The first entries you see are `user` and `assistant` which both have `message`.
**How to avoid:** Use the tagged enum pattern with per-variant structs. Only access `message` on types that have it.
**Warning signs:** Deserialization errors on progress entries.

### Pitfall 2: Confusing Byte Size with Token Count
**What goes wrong:** Using file size or JSON byte size as a proxy for token count.
**Why it happens:** Bytes and tokens feel correlated but the ratio varies (3.5-4.0 chars/token for English, lower for code).
**How to avoid:** Always prefer the `usage` field from the last assistant message for exact counts. Use byte sizes only for file-level metadata (in `context list`).
**Warning signs:** Diagnostics showing impossible context percentages.

### Pitfall 3: Not Filtering Sidechain Messages
**What goes wrong:** Including sidechain (subagent) messages in main context token count.
**Why it happens:** Sidechain entries look identical to main-chain entries but have `isSidechain: true`.
**How to avoid:** Always check `isSidechain` field on every entry. This field is at the top level of the JSONL entry, not inside `message`.
**Warning signs:** Token count far exceeding 200K context window.

### Pitfall 4: Multiple Assistant Entries Per Turn
**What goes wrong:** Counting tokens from every assistant entry when some are streaming partial responses.
**Why it happens:** Claude Code writes multiple assistant entries per turn -- partial (thinking), then complete (with tool_use/text and usage).
**How to avoid:** The `usage` field is only present on the final assistant entry of a turn (the one with `stop_reason`). For token extraction, walk backwards and take the first assistant entry with usage data.
**Warning signs:** Wildly inflated output token counts.

### Pitfall 5: Brittle Progress Entry Parsing
**What goes wrong:** Parsing fails when Claude Code adds new progress subtypes.
**Why it happens:** Progress entries have `data.type` that varies: `hook_progress`, `agent_progress`, `bash_progress`. New types will appear.
**How to avoid:** For bloat categorization, treat all progress entries as "progress ticks" regardless of subtype. Don't match on `data.type` exhaustively.
**Warning signs:** Unknown variant errors during deserialization.

### Pitfall 6: System Reminders Embedded in User Messages
**What goes wrong:** Missing system reminders in bloat detection because they appear inside message content, not as separate entries.
**Why it happens:** System reminders are injected as `<system-reminder>...</system-reminder>` tags within user message content strings. They are not separate JSONL entries.
**How to avoid:** Use regex to detect `<system-reminder>` tags within text content of all message types.
**Warning signs:** Bloat breakdown shows 0% for system reminders when they clearly exist.

## Code Examples

### JSONL Session Entry Structure (from live data)

Verified structure of key entry types from actual `~/.claude/projects/` JSONL files:

```
Entry types and their frequency (from 830-entry session):
  progress:              640 (77%)  -- hook_progress, agent_progress, bash_progress
  assistant:             103 (12%)  -- model responses with content blocks
  user:                   72 (9%)   -- user messages and tool results
  file-history-snapshot:   7 (1%)   -- file state snapshots
  queue-operation:         4 (<1%)  -- queue management
  system:                  3 (<1%)  -- compact_boundary, stop_hook_summary, turn_duration
  pr-link:                 1 (<1%)  -- PR reference
```

### Usage Field Structure (from live assistant entry)
```json
{
  "input_tokens": 3,
  "cache_creation_input_tokens": 3502,
  "cache_read_input_tokens": 26514,
  "cache_creation": {
    "ephemeral_5m_input_tokens": 0,
    "ephemeral_1h_input_tokens": 3502
  },
  "output_tokens": 13,
  "service_tier": "standard",
  "inference_geo": "not_available"
}
```

Total context size = `input_tokens + cache_creation_input_tokens + cache_read_input_tokens`

### Assistant Content Block Types
```
tool_use:  68 (most common -- tool invocations)
text:      26 (prose responses)
thinking:   9 (extended thinking, has signature field)
```

### Common Fields on All Entry Types
All JSONL entries share these top-level fields:
- `type` (string) -- entry type discriminator
- `uuid` (string) -- unique entry ID
- `timestamp` (ISO 8601 string)
- `sessionId` (UUID string)
- `parentUuid` (nullable string)
- `isSidechain` (boolean)
- `cwd` (string) -- working directory
- `version` (string) -- Claude Code version
- `gitBranch` (string, optional)

### Model Context Window Map
```rust
// Source: Cozempic tokens.py, verified against Anthropic docs
const MODEL_CONTEXT_WINDOWS: &[(&str, u64)] = &[
    ("claude-opus-4-6", 200_000),
    ("claude-opus-4-5", 200_000),
    ("claude-sonnet-4-6", 200_000),
    ("claude-sonnet-4-5", 200_000),
    ("claude-haiku-4-5", 200_000),
    ("claude-3-5-sonnet", 200_000),
    ("claude-3-5-haiku", 200_000),
    ("claude-3-opus", 200_000),
    ("claude-3-sonnet", 200_000),
    ("claude-3-haiku", 200_000),
];
const DEFAULT_CONTEXT_WINDOW: u64 = 200_000;
const SYSTEM_OVERHEAD_TOKENS: u64 = 21_000;
```

### Session Discovery via history.jsonl
```json
// Each line in ~/.claude/history.jsonl:
{
  "display": "/plugin marketplace add ...",
  "pastedContents": {},
  "project": "/Users/wollax/Git/personal/assay",
  "sessionId": "3201041c-df85-4c91-a485-7b8c189f7636",
  "timestamp": 1766983638489
}
```

### Bloat Category Detection Heuristics

| Category | Detection Method |
|----------|-----------------|
| Progress ticks | `type == "progress"` -- any subtype |
| Thinking blocks | Content blocks with `type == "thinking"` in assistant entries |
| Stale reads | Track `Read` tool_use inputs by `file_path`; re-reads of same path = stale |
| Tool output | Total bytes of tool_result content blocks (from user entries following tool_use) |
| Metadata | `file-history-snapshot`, `queue-operation`, `pr-link`, `system` entries |
| System reminders | Regex `<system-reminder>.*?</system-reminder>` in any text content |

### Stale Read Detection Heuristic (Claude's Discretion)

Recommended approach: Track `(file_path, offset, limit)` tuples from `Read` tool_use inputs across the session. A re-read of the same path with the same or overlapping range is "stale" -- the content was already in context. Count the bytes of the tool_result content for stale reads.

```rust
struct ReadTracker {
    seen: HashMap<String, Vec<ReadRange>>,  // path -> ranges read
}

struct ReadRange {
    offset: Option<u64>,
    limit: Option<u64>,
    entry_index: usize,
}

fn is_stale_read(&self, path: &str, offset: Option<u64>, limit: Option<u64>) -> bool {
    // If same path was read before with overlapping range, it's stale
    self.seen.get(path).map_or(false, |ranges| {
        ranges.iter().any(|r| ranges_overlap(r, offset, limit))
    })
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|---|---|---|---|
| Python-only tooling (Cozempic) | Rust native implementation | This phase | 10-100x faster parsing, native integration with assay |
| Single context window size (200K) | Model-aware context windows | Claude Code 2.x | Need model detection from session data |
| No caching awareness | Cache token breakdown | Anthropic API 2025 | Usage now includes cache_creation and cache_read fields |

**Deprecated/outdated:**
- Cozempic's `find_claude_pid` / `lsof` approach for session detection: Not needed for assay since we discover sessions by project directory, not by process tree.
- File-size-based overflow detection: Token-based detection is more accurate since JSONL overhead (progress ticks) inflates file size relative to actual context usage.

## Open Questions

1. **Claude Code JSONL schema stability**
   - What we know: The format includes `type`, `uuid`, `timestamp`, `sessionId` as stable fields. Entry types include at least 7 variants.
   - What's unclear: Whether Claude Code considers this a stable internal format or reserves the right to change it. No public documentation found.
   - Recommendation: Design defensively with `#[serde(other)]` catch-all variant and graceful degradation. Log warnings for unknown entry types rather than failing.

2. **Exact token count from last assistant vs. cumulative**
   - What we know: The `usage` field on the last assistant message represents that single API call's token usage. The `input_tokens + cache_creation_input_tokens + cache_read_input_tokens` total represents the full context sent to the API for that turn.
   - What's unclear: Whether this total is truly the "context window utilization" or if there's additional overhead (system prompts, tool definitions) not captured.
   - Recommendation: Use the total from the last assistant message as the primary metric. Cozempic adds a `SYSTEM_OVERHEAD_TOKENS = 21_000` constant for system prompt overhead. Consider making this configurable.

3. **`context list` column data availability**
   - What we know: File metadata (size, mtime, line count) is always available. Token count requires parsing at least the tail of each file. Model name is in assistant entries.
   - What's unclear: Whether parsing all session files for `context list` is acceptable performance-wise (could be 300+ files).
   - Recommendation: Show file metadata columns by default (fast). Add `--tokens` flag that also shows token counts (slower, uses tail-read optimization). Use `quick_token_estimate` (50KB tail read) rather than full parse.

## Sources

### Primary (HIGH confidence)
- Live `~/.claude/projects/-Users-wollax-Git-personal-assay/*.jsonl` files -- analyzed 830-entry session for structure
- Cozempic source code (GitHub: Ruya-AI/cozempic) -- session.py, tokens.py, diagnosis.py, helpers.py, types.py
- serde_json Context7 docs (/serde-rs/json) -- streaming deserialization patterns
- clap Context7 docs (/websites/rs_clap) -- nested subcommand derive API

### Secondary (MEDIUM confidence)
- Cozempic model context window map -- values match known Anthropic documentation but should be periodically verified
- `SYSTEM_OVERHEAD_TOKENS = 21_000` constant from Cozempic -- reasonable estimate but not officially documented

### Tertiary (LOW confidence)
- Claude Code JSONL format stability -- no public documentation; format inferred from live files and Cozempic reverse engineering
- `history.jsonl` as session index -- undocumented internal file, may change

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- all dependencies already in workspace except dirs (trivial addition)
- Architecture: HIGH -- clear patterns from Cozempic, live session files analyzed, workspace conventions established
- Pitfalls: HIGH -- verified against real session data; Cozempic source reveals same pitfalls and mitigations
- JSONL format: MEDIUM -- reverse-engineered from live data and Cozempic, not officially documented

**Research date:** 2026-03-06
**Valid until:** 2026-04-06 (30 days -- Claude Code JSONL format could change with updates)
