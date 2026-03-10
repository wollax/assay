# Phase 29: Gate Output Truncation - Research

**Researched:** 2026-03-09
**Domain:** Rust string processing, UTF-8 safe truncation, process output capture
**Confidence:** HIGH

## Summary

This phase replaces the existing tail-only `truncate_output` function in `assay-core::gate` with a head+tail truncation engine that preserves both the beginning and end of command output, separated by a `[truncated: X bytes omitted]` marker. The domain is well-understood: it is pure Rust standard library string/byte manipulation with no external dependencies needed.

The codebase already has the scaffolding in place: `GateResult.truncated` and `GateResult.original_bytes` fields exist, the `evaluate_command` function already captures stdout/stderr into `Vec<u8>` buffers, converts via `String::from_utf8_lossy`, and applies truncation post-capture. The work is to replace the truncation function with a head+tail variant, change the per-stream budget from 64 KiB to 32 KiB, and ensure `original_bytes` tracks per-stream sizes correctly.

**Primary recommendation:** Build a pure function `truncate_head_tail(input: &str, budget: usize) -> TruncationResult` that operates on already-captured UTF-8 strings, using `str::floor_char_boundary` and `str::ceil_char_boundary` for safe slicing. Keep the post-capture approach (not streaming). The function should be independent of gate evaluation — a standalone utility with comprehensive unit tests.

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Rust std `str` | 1.93.0 | `floor_char_boundary`, `ceil_char_boundary` | Stable since 1.73.0, already used in codebase |
| Rust std `String` | 1.93.0 | `from_utf8_lossy` for byte-to-string conversion | Already used in `evaluate_command` |

### Supporting

No additional libraries needed. This is purely standard library work.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `str::floor_char_boundary` | Manual UTF-8 byte scanning | No reason — stdlib is correct and stable |
| Post-capture truncation | Streaming ring buffer | Unnecessary complexity for 32 KiB budget; full capture already works |
| `from_utf8_lossy` | `from_utf8` with error handling | Lossy is correct — gate output may contain non-UTF-8 bytes |

**Installation:** No new dependencies. Zero workspace dependency constraint satisfied.

## Architecture Patterns

### Where Truncation Lives

The truncation function should live in `crates/assay-core/src/gate/mod.rs` alongside the existing `truncate_output` function it replaces. It is an internal helper, not a public API surface.

```
crates/assay-core/src/gate/
├── mod.rs          # evaluate_command calls truncate_head_tail
│                   # truncate_head_tail defined here (replaces truncate_output)
└── session.rs      # Unchanged
```

If the function grows complex enough to warrant its own file (unlikely), it could move to `gate/truncation.rs`, but starting inline is simpler and matches the existing pattern.

### Pattern 1: Pure Truncation Function

**What:** A stateless function that takes a string and byte budget, returns the truncated string plus metadata.
**When to use:** After command output is fully captured and converted to UTF-8.

```rust
/// Result of applying head+tail truncation to a string.
struct TruncationResult {
    /// The possibly-truncated output (head + marker + tail, or original if within budget).
    output: String,
    /// Whether truncation was applied.
    truncated: bool,
    /// Original byte length before truncation.
    original_bytes: usize,
}

/// Truncate output using head+tail strategy with a byte budget.
///
/// If `input.len() <= budget`, returns input unchanged.
/// Otherwise, splits the budget between head and tail sections,
/// finds UTF-8-safe boundaries, and joins with a truncation marker.
fn truncate_head_tail(input: &str, budget: usize) -> TruncationResult {
    if input.len() <= budget {
        return TruncationResult {
            output: input.to_string(),
            truncated: false,
            original_bytes: input.len(),
        };
    }

    let head_budget = budget / 3;         // ~33% for head
    let tail_budget = budget - head_budget; // ~67% for tail (errors at end)

    // Find UTF-8-safe boundaries
    let head_end = input.floor_char_boundary(head_budget);
    let tail_start = input.ceil_char_boundary(input.len() - tail_budget);

    let omitted = input.len() - head_end - (input.len() - tail_start);
    let marker = format!("\n[truncated: {} bytes omitted]\n", omitted);

    TruncationResult {
        output: format!("{}{}{}", &input[..head_end], marker, &input[tail_start..]),
        truncated: true,
        original_bytes: input.len(),
    }
}
```

### Pattern 2: Per-Stream Application

**What:** Apply truncation independently to stdout and stderr, then populate `GateResult` fields.
**When to use:** In `evaluate_command` after joining reader threads.

```rust
let stdout_result = truncate_head_tail(&stdout_raw, STREAM_BUDGET);
let stderr_result = truncate_head_tail(&stderr_raw, STREAM_BUDGET);

let truncated = stdout_result.truncated || stderr_result.truncated;
let original_bytes = if truncated {
    Some((stdout_result.original_bytes + stderr_result.original_bytes) as u64)
} else {
    None
};
```

### Anti-Patterns to Avoid

- **Truncating raw bytes then converting to UTF-8:** Truncation must happen on the UTF-8 string, not the raw `Vec<u8>`, because `from_utf8_lossy` may change byte count (replacing invalid sequences with U+FFFD).
- **Using `String::truncate`:** This panics if the index is not on a char boundary. Use `floor_char_boundary` instead.
- **Streaming/ring buffer approach:** Over-engineering for a 32 KiB budget. The process output is already fully captured into memory; truncating post-hoc is simpler and correct.
- **Making the truncation marker configurable:** The marker format is specified in GATE-02 (`[truncated: X bytes omitted]`). Don't parameterize what's fixed.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| UTF-8-safe string slicing | Manual byte-offset scanning | `str::floor_char_boundary` / `str::ceil_char_boundary` | Stable since 1.73.0, handles all edge cases correctly |
| Byte-to-string conversion | Custom UTF-8 validation | `String::from_utf8_lossy` | Already in use, handles binary output gracefully |
| Process output capture | Custom pipe draining | Existing reader thread pattern | Already correct, handles deadlock prevention |

**Key insight:** The entire truncation domain fits in ~30 lines of code using Rust's standard library. The complexity is in edge cases (empty input, budget smaller than marker, budget = 0, input exactly at budget boundary, multi-byte chars at split points), which require comprehensive tests, not complex code.

## Common Pitfalls

### Pitfall 1: Marker Counted Against Budget

**What goes wrong:** The truncation marker (`[truncated: X bytes omitted]\n`) itself consumes bytes. If you subtract marker size from the budget, the head+tail sections shrink. If you don't, the total output exceeds the budget.
**Why it happens:** Ambiguity about whether "budget" means "max output including marker" or "max content bytes."
**How to avoid:** Decide upfront. Recommendation: budget counts the *content* (head+tail), not the marker. The marker is overhead that makes the total output slightly larger than the budget. This is simpler to implement and the few extra bytes of marker text are negligible against a 32 KiB budget.
**Warning signs:** Tests that check `output.len() <= budget` will fail if marker is not subtracted.

### Pitfall 2: Head/Tail Overlap When Input Is Barely Over Budget

**What goes wrong:** If input is only slightly over budget, `head_end` and `tail_start` may overlap or be equal, producing zero omitted bytes or a negative count.
**Why it happens:** `head_budget + tail_budget >= input.len()` when input barely exceeds the budget.
**How to avoid:** Check for overlap: if `tail_start <= head_end`, just return the input unchanged (or truncated from one end only). Alternatively, ensure the function is only called when `input.len() > budget`.
**Warning signs:** Marker shows "[truncated: 0 bytes omitted]" or panics on negative arithmetic.

### Pitfall 3: `original_bytes` Semantics Change

**What goes wrong:** Currently `original_bytes` is `Some((stdout_len + stderr_len) as u64)` — a combined total. This is ambiguous when one stream is truncated and the other is not.
**Why it happens:** The field was added as a placeholder before per-stream truncation was designed.
**How to avoid:** Keep `original_bytes` as combined total (backward compatible) but document clearly. Per-stream tracking can be added later if needed. The `truncated: bool` flag already exists and indicates "at least one stream was truncated."
**Warning signs:** Consumers misinterpreting `original_bytes` as a single stream's size.

### Pitfall 4: Empty Input Edge Case

**What goes wrong:** Calling `floor_char_boundary(0)` or `ceil_char_boundary(0)` on empty string.
**Why it happens:** Gate commands that produce no output.
**How to avoid:** Early return for empty input — no truncation needed, `truncated: false`.
**Warning signs:** Off-by-one errors or unnecessary marker insertion on empty strings.

### Pitfall 5: Budget Too Small for Head+Tail+Marker

**What goes wrong:** If budget is very small (e.g., 10 bytes), there's no room for meaningful head and tail sections.
**Why it happens:** Defensive testing with tiny budgets.
**How to avoid:** The function should still work correctly with tiny budgets — it just means the head and tail are very short. The marker itself is never suppressed; if budget is 0, the output is just the marker. This is an edge case for tests, not production (32 KiB is plenty).
**Warning signs:** Panics or empty output with small budgets.

## Code Examples

### UTF-8 Safe Slicing with Standard Library

```rust
// floor_char_boundary: round DOWN to nearest char boundary
let s = "Hello, 世界!"; // 13 bytes
assert_eq!(s.floor_char_boundary(8), 7);  // Before '世' (3-byte char at 7..10)
assert_eq!(&s[..7], "Hello, ");

// ceil_char_boundary: round UP to nearest char boundary
assert_eq!(s.ceil_char_boundary(8), 10);  // After '世'
assert_eq!(&s[10..], "界!");
```

### Head+Tail Truncation with Multi-Byte Safety

```rust
fn truncate_head_tail(input: &str, budget: usize) -> TruncationResult {
    if input.len() <= budget {
        return TruncationResult {
            output: input.to_string(),
            truncated: false,
            original_bytes: input.len(),
        };
    }

    // Tail-biased: errors and failures tend to appear at the end
    let head_budget = budget / 3;
    let tail_budget = budget - head_budget;

    let head_end = input.floor_char_boundary(head_budget);
    let tail_start = input.ceil_char_boundary(input.len().saturating_sub(tail_budget));

    // Guard against overlap (input barely over budget)
    if tail_start <= head_end {
        // Just use tail-only truncation as fallback
        let start = input.ceil_char_boundary(input.len().saturating_sub(budget));
        let omitted = start;
        return TruncationResult {
            output: format!(
                "[truncated: {} bytes omitted]\n{}",
                omitted,
                &input[start..]
            ),
            truncated: true,
            original_bytes: input.len(),
        };
    }

    let omitted = tail_start - head_end;
    TruncationResult {
        output: format!(
            "{}\n[truncated: {} bytes omitted]\n{}",
            &input[..head_end],
            omitted,
            &input[tail_start..]
        ),
        truncated: true,
        original_bytes: input.len(),
    }
}
```

### Key Test Cases

```rust
#[test]
fn truncation_preserves_utf8_boundaries() {
    // 3-byte UTF-8 chars: each '世' is 3 bytes
    let input = "世".repeat(100); // 300 bytes
    let result = truncate_head_tail(&input, 100);
    assert!(result.truncated);
    // Verify the output is valid UTF-8 (would panic if not)
    let _ = result.output.as_bytes();
    // Verify no partial chars: all chars should be complete '世' or marker text
    for c in result.output.chars() {
        assert!(c == '世' || c.is_ascii(), "unexpected char: {c:?}");
    }
}

#[test]
fn truncation_independent_streams() {
    // stdout: large, gets truncated
    // stderr: small, not truncated
    let big_stdout = "x".repeat(100_000);
    let small_stderr = "warning: something";

    let stdout_result = truncate_head_tail(&big_stdout, 32_768);
    let stderr_result = truncate_head_tail(small_stderr, 32_768);

    assert!(stdout_result.truncated);
    assert!(!stderr_result.truncated);
}

#[test]
fn marker_format_matches_spec() {
    let input = "a".repeat(200);
    let result = truncate_head_tail(&input, 100);
    assert!(result.output.contains("[truncated: "));
    assert!(result.output.contains(" bytes omitted]"));
}
```

## State of the Art

| Old Approach (current) | New Approach (this phase) | Impact |
|------------------------|--------------------------|--------|
| Tail-only truncation | Head+tail with marker | Preserves context from both beginning and end of output |
| 64 KiB per stream | 32 KiB per stream | Reduces worst-case storage, matches CONTEXT.md decision |
| `[truncated, showing last N bytes]` marker | `[truncated: X bytes omitted]` marker | Matches GATE-02 requirement |
| `original_bytes` = combined total | `original_bytes` = combined total (unchanged) | Backward compatible |

**Deprecated/outdated:**
- The existing `truncate_output` function and `MAX_OUTPUT_BYTES` constant will be replaced.

## Design Decisions (Claude's Discretion)

### Head/Tail Ratio: 1:2 (33% head, 67% tail)

**Rationale:** Error messages, stack traces, and test failures appear at the end of output. The beginning typically contains build preamble, compilation progress, or test setup. A tail-biased split preserves the most diagnostic value. The 1:2 ratio gives ~10.7 KiB for head and ~21.3 KiB for tail within a 32 KiB budget — plenty of context from both ends.

### Budget Target: Byte-based with UTF-8 alignment

**Rationale:** Budget counts raw string bytes (which are UTF-8 bytes in Rust). Truncation boundaries are adjusted to the nearest UTF-8 character boundary using `floor_char_boundary`/`ceil_char_boundary`. This means the actual head+tail content may be slightly less than the budget (by at most 3 bytes for a 4-byte UTF-8 sequence boundary adjustment), but never more.

### Truncation Timing: Post-capture

**Rationale:** The existing `evaluate_command` already reads all output into `Vec<u8>`, converts to `String`, then truncates. This is correct and simple. A streaming ring buffer would add complexity for no benefit — 32 KiB is small enough that capturing the full output first is always fine. Even commands producing megabytes of output will just read into memory briefly before truncation discards the middle.

### Binary/Non-UTF-8 Handling: Lossy conversion (existing behavior)

**Rationale:** `String::from_utf8_lossy` is already used and is the right choice. Invalid UTF-8 sequences are replaced with U+FFFD (3 bytes each), which may slightly change the byte count. Truncation operates on the resulting valid UTF-8 string, not the raw bytes. This is correct because the `original_bytes` field should reflect the pre-truncation UTF-8 string length, not the raw process output length.

### Per-Gate Configurability: Not in this phase

**Rationale:** The 32 KiB default is appropriate for all current use cases. Per-gate budget configuration can be added later by extending the `[gate]` TOML section. For now, use a constant.

### Empty Output: Return empty string, truncated=false

**Rationale:** If a command produces no stdout or stderr, the truncation function returns an empty string with `truncated: false` and `original_bytes: 0`. This matches existing behavior where empty strings are skipped in serialization.

### Backward Compatibility: No migration needed

**Rationale:** The `truncated` and `original_bytes` fields already exist with `skip_serializing_if` guards. Old records without these fields will deserialize with defaults (`false` and `None`). New records with head+tail truncation will have the marker embedded in the stdout/stderr string itself. No schema migration is needed.

## Open Questions

1. **Should `original_bytes` track per-stream sizes?**
   - What we know: Currently a single combined `u64`. GATE-05 says "reflects the pre-truncation size" (singular).
   - What's unclear: Whether consumers need to know which stream was truncated and by how much.
   - Recommendation: Keep combined for now. The `truncated: bool` flag plus the marker in the string content provide enough information. Per-stream tracking (e.g., `original_stdout_bytes`/`original_stderr_bytes`) would require schema changes and can be deferred.

2. **Should the marker include line counts?**
   - What we know: GATE-02 specifies `[truncated: X bytes omitted]`. No mention of lines.
   - What's unclear: Whether including `(~N lines)` would aid debugging.
   - Recommendation: Stick to the spec. Bytes-only marker. Line counting adds complexity for marginal value.

## Sources

### Primary (HIGH confidence)
- Codebase analysis: `crates/assay-types/src/gate.rs` — GateResult struct with truncated/original_bytes fields
- Codebase analysis: `crates/assay-core/src/gate/mod.rs` — existing `truncate_output`, `evaluate_command`, reader thread pattern
- Rust std docs (Context7: `/websites/doc_rust-lang_stable_std`) — `str::floor_char_boundary`, `str::ceil_char_boundary` stable and documented

### Secondary (MEDIUM confidence)
- None needed — this is a well-understood domain with no external dependencies

### Tertiary (LOW confidence)
- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — Rust std only, no dependencies, APIs verified as stable
- Architecture: HIGH — Replacing existing function with same calling pattern
- Pitfalls: HIGH — Well-known edge cases in string truncation, all addressable with tests

**Research date:** 2026-03-09
**Valid until:** 2026-06-09 (stable domain, no external dependencies to version-shift)
