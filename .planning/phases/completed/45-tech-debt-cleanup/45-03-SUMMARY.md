---
plan: 45-03
status: complete
wave: 1
completed: 2026-03-15
commits:
  - 9b2a6e2  # Task 1: truncation fixes
  - 265d9b6  # Task 2: gate command helpers
issues_resolved: 15
---

# 45-03 Summary: assay-core truncation + gate sweep

## What was done

### Task 1: Truncation fixes and derives (assay-core/gate)

- **`TruncationResult` derives `Debug`** — added `#[derive(Debug)]`
- **`truncate_head_tail` allocation fix** — changed signature from `input: &str` to `input: String`; on the passthrough path the original `String` is returned without a new allocation
- **`debug_assert!` invariant** — added assertion that `head.len() + tail.len() <= original_bytes` before the subtraction in `truncate_head_tail`
- **`EVIDENCE_DISPLAY_CHARS` constant** — replaced magic `200` in gate history detail view with named constant; also fixed evidence vs reasoning char/byte inconsistency (both now use `.chars().count()`)
- **`spec_entry_gate_info` helper** — extracted duplicated `SpecEntry` match blocks for gate_section + criteria into a single helper in `gate.rs`
- **`is_executable` helper** — extracted duplicated `c.cmd.is_some() || c.path.is_some()` predicate into named helper in `gate.rs`
- **Tests added:**
  - `truncate_head_tail_marker_format` — strengthened: asserts marker is preceded and followed by `\n`
  - `truncate_head_tail_multiline_input` — tests multi-line input truncation and that head+tail == budget
  - `truncate_head_tail_over_budget` — strengthened: asserts exact omitted byte count and head/tail sizes

### Task 2: Gate command helpers and remaining fixes

- **`CommandErrorKind` derives `Hash`** — enables use in `HashSet`/`HashMap`
- **`enriched_error_display` → `format_enriched_error`** — renamed to follow `format_*` naming convention; all call sites updated (assay-core and assay-cli)
- **Levenshtein optimization** — `b.chars()` collected into `Vec<char>` upfront so inner loop uses O(1) indexing instead of re-iterating the string on each outer iteration
- **`gate_exit_code` doc cross-reference** — added `/// See also: classify_exit_code` doc comment
- **Tests added:**
  - `classify_exit_code_boundaries` — tests codes 0, 1, 126, 127, 128, 255
  - `levenshtein_transposition` — tests "ab"↔"ba", "abc"↔"bac", "abcde"↔"bacde"

### Pre-existing fixes (not in plan scope, but blocking `just ready`)

- **`RecoverySummary` missing `Eq` derive** — clippy warning pre-dated this plan
- **Missing `use crate::AssayError` in config test module** — compile error pre-dated this plan (introduced in 45-02)

## Issues resolved

| Issue | Resolution |
|-------|-----------|
| `truncation-result-missing-debug` | Added `#[derive(Debug)]` |
| `truncate-head-tail-unnecessary-alloc` | Changed to accept `String` by value |
| `truncate-omitted-debug-assert` | Added `debug_assert!` for invariant |
| `truncate-marker-newline-test` | Strengthened existing marker test |
| `truncate-multiline-input-test` | Added new multiline test |
| `truncate-over-budget-test-assertions` | Strengthened over-budget test |
| `evidence-truncation-magic-number` | Named constant + char/byte fix |
| `spec-entry-criteria-extraction-dup` | Extracted `spec_entry_gate_info` helper |
| `command-error-kind-derive-hash` | Added `Hash` derive |
| `enriched-error-display-rename` | Renamed to `format_enriched_error` |
| `is-executable-filter-repeated` | Extracted `is_executable` helper |
| `levenshtein-collect-chars-upfront` | Pre-collect `b_chars: Vec<char>` |
| `classify-exit-code-boundary-tests` | Added boundary test |
| `gate-exit-code-doc-cross-ref` | Added doc comment |
| `levenshtein-transposition-test` | Added transposition test |

## Verification

- `just ready` passes (fmt-check + lint + test + deny)
- 527 assay-core tests pass, 26 assay-types snapshot tests pass
- `TruncationResult` is Debug-printable
- `CommandErrorKind` is Hash-able
- Levenshtein uses `Vec<char>` for O(1) indexing
