# Phase 31: Error Messages — Research

**Researched:** 2026-03-10
**Confidence:** HIGH (all code paths traced, toml crate API verified)

## Standard Stack

No new dependencies needed. Everything is achievable with:
- `std::io::ErrorKind` — for command-not-found differentiation
- `toml` 0.8 (already in workspace) — `toml::de::Error` has `.span()` and `.message()` methods
- Hand-rolled Levenshtein distance — trivial ~15-line function, no crate needed
- Existing `assay_core::spec::scan()` — already enumerates all specs

## Architecture Patterns

### ERR-01: Command-not-found differentiation

**Current code path:** `crates/assay-core/src/gate/mod.rs`, function `evaluate_command()` (line 372). The spawn failure is:

```rust
let mut child = command
    .spawn()
    .map_err(|source| AssayError::GateExecution {
        cmd: cmd.to_string(),
        working_dir: working_dir.to_path_buf(),
        source,
    })?;
```

The `GateExecution` variant stores `source: std::io::Error`. The current `Display` impl is:
```
gate execution failed for `{cmd}` in `{working_dir}`: {source}
```

**Implementation pattern:** Do NOT change the `AssayError::GateExecution` variant or its Display impl. Instead, add a method on `AssayError` (or a standalone function) that formats the user-facing message by inspecting `source.kind()`:

```rust
impl AssayError {
    /// Format a user-facing message for gate execution errors.
    /// Differentiates NotFound, PermissionDenied, and generic IO errors.
    pub fn gate_execution_display(&self) -> String {
        match self {
            Self::GateExecution { cmd, source, .. } => {
                let binary = cmd.split_whitespace().next().unwrap_or(cmd);
                match source.kind() {
                    std::io::ErrorKind::NotFound => {
                        format!("command '{binary}' not found. Is it installed and in PATH?")
                    }
                    std::io::ErrorKind::PermissionDenied => {
                        format!("command '{binary}' found but not executable. Check file permissions.")
                    }
                    _ => self.to_string(),
                }
            }
            _ => self.to_string(),
        }
    }
}
```

**Consumers that must use the new method:**
1. `crates/assay-cli/src/commands/gate.rs` — `stream_criterion()` function, line 278: `eprintln!("    error: {err}");`
2. `crates/assay-core/src/gate/mod.rs` — `evaluate_all_inner()` function, line 226: `format!("gate evaluation error: {err}")`
3. `crates/assay-mcp/src/server.rs` — `domain_error()` function, line 1100: `err.to_string()`

**Binary extraction:** `cmd.split_whitespace().next().unwrap_or(cmd)` — handles `"cargo test"` -> `"cargo"`, `"sh"` -> `"sh"`, empty string -> empty string.

**Working directory omission:** For `NotFound` and `PermissionDenied`, the working directory is irrelevant noise. The new format deliberately omits it. For other errors, the existing format (which includes working_dir) is preserved.

### ERR-02: Spec-not-found diagnostics

**Current code path:** `crates/assay-core/src/spec/mod.rs`, function `load_spec_entry()` (line 275):

```rust
Err(AssayError::SpecNotFound {
    name: slug.to_string(),
    specs_dir: specs_dir.to_path_buf(),
})
```

Current Display: `spec '{name}' not found in {specs_dir}`

**Implementation pattern:** Enrich the `SpecNotFound` variant with available spec information. Two approaches:

**Approach A (recommended): Add a helper function that enriches the error message.** Add a public function in `assay_core::spec` that wraps `load_spec_entry` with diagnostics:

```rust
/// Load a spec entry, enriching SpecNotFound errors with available spec list and fuzzy match.
pub fn load_spec_entry_with_diagnostics(slug: &str, specs_dir: &Path) -> Result<SpecEntry> {
    match load_spec_entry(slug, specs_dir) {
        Ok(entry) => Ok(entry),
        Err(AssayError::SpecNotFound { name, specs_dir }) => {
            // scan for available specs
            let (available, invalid) = list_available_specs(&specs_dir);
            let fuzzy_match = find_fuzzy_match(&name, &available);
            Err(AssayError::SpecNotFoundDiagnostic {
                name,
                specs_dir,
                available,
                invalid,
                suggestion: fuzzy_match,
            })
        }
        Err(other) => Err(other),
    }
}
```

**Approach B: Add fields to existing SpecNotFound.** This changes the variant signature, but since `#[non_exhaustive]` is set, downstream code uses wildcards anyway. However, this forces all SpecNotFound construction sites to populate the extra fields, which is messy when the scan isn't always desired.

**Recommendation: Approach A** — add a new variant `SpecNotFoundDiagnostic` with extra fields. Keep `SpecNotFound` for internal use. The new variant's Display renders:

- With specs: `spec 'auth-flow' not found. Available specs: login, signup, billing`
- With 10+ specs: `spec 'x' not found. Available specs: a, b, c, d, e, f, g, h, i, j (and 5 more)`
- Zero specs: `No specs found in .assay/specs/.`
- Invalid specs: `billing (invalid)` in the list + separate warning
- Fuzzy match: `Did you mean 'auth-flow'?` appended if one close match

**Available spec enumeration:** Use `assay_core::spec::scan()` which returns `ScanResult { entries, errors }`. Extract slugs from `entries` and error filenames from `errors`.

**Call sites that construct or catch SpecNotFound:**
1. `crates/assay-cli/src/commands/gate.rs` — lines 446, 600, 724: catches `SpecNotFound`, reformats with `bail!`
2. `crates/assay-cli/src/commands/spec.rs` — line 58: catches `SpecNotFound`, reformats with `bail!`
3. `crates/assay-mcp/src/server.rs` — `load_spec_entry_mcp()` line 1081: passes through `domain_error()`

All CLI call sites currently swallow the original error and `bail!` with a custom message. These should switch to calling `load_spec_entry_with_diagnostics` and letting the enriched error propagate.

### ERR-03: TOML parse error enrichment

**Current code paths for TOML parsing:**

1. **Config:** `crates/assay-core/src/config/mod.rs` line 88:
   ```rust
   let config: Config = toml::from_str(&content).map_err(|e| AssayError::ConfigParse {
       path: path.clone(),
       message: e.to_string(),
   })?;
   ```

2. **Legacy spec:** `crates/assay-core/src/spec/mod.rs` line 211:
   ```rust
   let spec: Spec = toml::from_str(&content).map_err(|e| AssayError::SpecParse {
       path: path.to_path_buf(),
       message: e.to_string(),
   })?;
   ```

3. **Gates spec:** `crates/assay-core/src/spec/mod.rs` `load_gates()`:
   ```rust
   let gates: GatesSpec = toml::from_str(&content).map_err(|e| AssayError::GatesSpecParse {
       path: path.to_path_buf(),
       message: e.to_string(),
   })?;
   ```

4. **Feature spec:** `crates/assay-core/src/spec/mod.rs` `load_feature_spec()`:
   ```rust
   let spec: FeatureSpec = toml::from_str(&content).map_err(|e| AssayError::FeatureSpecParse {
       path: path.to_path_buf(),
       message: e.to_string(),
   })?;
   ```

**Key finding: `toml::de::Error::Display` already includes rich formatting.** When calling `e.to_string()` on a `toml::de::Error`, the output already contains:
- Line/column numbers
- The offending source line
- A caret pointer

Example output from `toml::de::Error::to_string()`:
```
TOML parse error at line 1, column 10
  |
1 | 00:32:00.a999999
  |          ^
Unexpected `a`
Expected `digit`
```

**This means the existing error messages already contain line/column and source line information** — they are stored in the `message: String` field of each parse error variant. The `toml` crate's `Display` impl does this automatically because `toml::from_str` (as opposed to `toml::de::Deserializer::new(input).deserialize()`) retains the input in the error.

**What's missing:**
1. The `Display` impls for `SpecParse`, `ConfigParse`, etc. prepend `parsing spec '{path}': {message}` which puts the file path at the start, but the toml error's multi-line formatting may look awkward when concatenated.
2. The file path display could be more prominent.
3. Source lines longer than ~80 chars are not truncated.

**Implementation pattern:** Replace the simple `e.to_string()` call with a custom formatting function that:
1. Extracts span from `toml::de::Error::span()` (returns `Option<Range<usize>>`)
2. Extracts message from `toml::de::Error::message()` (returns `&str`)
3. Uses the already-loaded `content: &str` to index into the source
4. Formats with file path prominently shown, truncated source line, and caret pointer

```rust
fn format_toml_error(path: &Path, content: &str, err: &toml::de::Error) -> String {
    let message = err.message();
    let Some(span) = err.span() else {
        return format!("{message}");
    };
    // Calculate line/column from span.start
    let (line, col) = translate_position(content, span.start);
    let source_line = content.lines().nth(line).unwrap_or("");
    let truncated = truncate_source_line(source_line, col, 80);
    format!(
        "line {}, column {}: {message}\n  |\n{} | {}\n  | {}^",
        line + 1, col + 1,
        line + 1, truncated.text,
        " ".repeat(truncated.caret_offset),
    )
}
```

The `translate_position` function is straightforward (count newlines before the byte offset).

**Affected error variants and their Display impls:**
- `ConfigParse` — `parsing config '{path}': {message}`
- `SpecParse` — `parsing spec '{path}': {message}`
- `GatesSpecParse` — `parsing gates spec '{path}': {message}`
- `FeatureSpecParse` — `parsing feature spec '{path}': {message}`

All four share the same pattern. The fix applies identically to all four.

## Don't Hand-Roll

- **TOML parser** — use `toml` 0.8 (already in workspace)
- **Error derive** — use `thiserror` (already in workspace)
- **Process spawning** — use `std::process::Command` (already used)
- **Spec scanning** — use existing `assay_core::spec::scan()` function

## Hand-Roll (simple enough, no crate needed)

- **Levenshtein distance** — ~15 lines, standard dynamic programming algorithm. Use a 1D vector for space efficiency. Threshold: suggest match only if distance <= 2 AND distance < name.len() / 2 (to avoid suggesting "a" for "b").
- **Source line truncation** — center the error column in an ~80 char window, add `...` at truncation points.
- **Position translation** — count newlines in `content[..byte_offset]` to get line number, then subtract last newline position to get column.

## Common Pitfalls

1. **`sh -c` wrapping hides binary name:** The spawn command is `sh -c "cargo test"`, so `io::ErrorKind::NotFound` means `sh` was not found, not `cargo`. However, on any standard Unix system `sh` always exists. The `NotFound` error from `Command::new("sh").args(["-c", cmd]).spawn()` would only fire if `sh` itself is missing, which is essentially impossible. **Wait** — this is a critical finding. Because all commands are spawned via `sh -c`, the `NotFound` from `.spawn()` means `sh` was not found, not the user's binary. If `cargo` is not found, `sh` will still spawn successfully, and the exit code will be non-zero (127) with stderr like `sh: cargo: not found`. This means **ERR-01 as designed only fires when `sh` itself is missing**, which is near-impossible on Unix.

   **Revised approach for ERR-01:** Check the exit code. Exit code 127 from a shell means "command not found" and exit code 126 means "permission denied". Detect these in the `GateResult` (where `exit_code == Some(127)`) and format the error message accordingly. The binary name extraction from the `cmd` string is still needed.

   **Alternative:** Use `Command::new(binary).args(rest)` instead of `sh -c`. But this would break commands that use shell features (pipes, redirects, globs). Not viable.

   **Recommendation:** Handle both paths:
   - `.spawn()` failure with `NotFound`/`PermissionDenied` → for when `sh` itself is missing (rare but correct)
   - Exit code 127/126 → for when the user's binary is not found/not executable (the common case)
   - For exit 127: check stderr for the standard shell error pattern to extract the actual missing command name

2. **Scan during SpecNotFound may fail:** `scan()` reads the directory and parses files. If the directory doesn't exist, `scan()` returns `Err(SpecScan)`. Handle gracefully — if scan fails, fall back to the bare "not found" message.

3. **Empty spec directory edge case:** When `scan()` returns zero entries and zero errors, display `"No specs found in {specs_dir}."` — don't attempt fuzzy matching.

4. **toml::de::Error span may be None:** For some deserialization errors (e.g., missing required fields), the span may not be available. Fall back to just showing the error message without source line display.

5. **Unicode in TOML files:** Column calculation must handle multi-byte characters. The `toml` crate's internal `translate_position` counts chars, not bytes. Our reimplementation should do the same.

6. **Test existing behavior first:** The `toml::de::Error::to_string()` already includes rich formatting. Verify that the current output is truly inadequate before adding custom formatting. It may be sufficient to just improve the prefix (file path display) without reimplementing the source line display.

7. **MCP error surface is `domain_error()`:** All errors flow through `fn domain_error(err: &AssayError) -> CallToolResult` at line 1100 in `server.rs`, which calls `err.to_string()`. Improving the `Display` impl or adding helper methods automatically improves MCP output — no separate MCP changes needed for most cases.

## Code Examples

### Levenshtein distance (hand-rolled)

```rust
/// Compute Levenshtein edit distance between two strings.
fn levenshtein(a: &str, b: &str) -> usize {
    let b_len = b.chars().count();
    let mut prev: Vec<usize> = (0..=b_len).collect();
    let mut curr = vec![0; b_len + 1];

    for (i, ca) in a.chars().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b.chars().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            curr[j + 1] = (prev[j] + cost)
                .min(prev[j + 1] + 1)
                .min(curr[j] + 1);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b_len]
}
```

**Threshold recommendation:** Suggest a match only when:
- `distance <= 2` AND
- `distance <= name.len() / 2` (avoids suggesting for very short names)
- Only one candidate meets the threshold (multiple close matches = no suggestion)

### Source line truncation

```rust
struct TruncatedLine {
    text: String,
    caret_offset: usize,
}

fn truncate_source_line(line: &str, col: usize, budget: usize) -> TruncatedLine {
    let chars: Vec<char> = line.chars().collect();
    if chars.len() <= budget {
        return TruncatedLine { text: line.to_string(), caret_offset: col };
    }
    let ellipsis = "...";
    let available = budget - ellipsis.len() * 2; // room for ... on each side
    let half = available / 2;
    let start = col.saturating_sub(half);
    let end = (start + available).min(chars.len());
    let start = end.saturating_sub(available); // re-adjust if we hit the end
    let prefix = if start > 0 { ellipsis } else { "" };
    let suffix = if end < chars.len() { ellipsis } else { "" };
    let slice: String = chars[start..end].iter().collect();
    let caret_offset = col - start + prefix.len();
    TruncatedLine { text: format!("{prefix}{slice}{suffix}"), caret_offset }
}
```

### Exit code 127/126 detection for ERR-01

```rust
// In gate result processing (not spawn error handling):
if let Some(exit_code) = result.exit_code {
    match exit_code {
        127 => {
            let binary = cmd.split_whitespace().next().unwrap_or(cmd);
            // stderr typically contains: "sh: <binary>: not found"
            format!("command '{binary}' not found. Is it installed and in PATH?")
        }
        126 => {
            let binary = cmd.split_whitespace().next().unwrap_or(cmd);
            format!("command '{binary}' found but not executable. Check file permissions.")
        }
        _ => { /* normal failure handling */ }
    }
}
```

## Key Findings Summary

| Finding | Confidence | Impact |
|---------|-----------|--------|
| Commands spawn via `sh -c`, so `io::ErrorKind::NotFound` means sh is missing, not user binary | HIGH | ERR-01 must check exit code 127/126 instead of (or in addition to) spawn error kind |
| `toml::de::Error::to_string()` already includes source line + caret | HIGH | ERR-03 may need less custom formatting than expected; focus on file path prominence |
| `domain_error()` in MCP uses `err.to_string()` | HIGH | Improving Display impls automatically improves MCP surface |
| Zero new deps needed | HIGH | Levenshtein is trivial to hand-roll |
| `scan()` returns both entries and errors | HIGH | ERR-02 can show invalid specs with markers from scan errors |
| All CLI SpecNotFound handlers use `bail!()` | HIGH | Must update 4 call sites to use enriched error |
| `#[non_exhaustive]` on AssayError | HIGH | New variants are non-breaking |

## Files to Modify

| File | Changes |
|------|---------|
| `crates/assay-core/src/error.rs` | Add `SpecNotFoundDiagnostic` variant; add `gate_execution_display()` method |
| `crates/assay-core/src/spec/mod.rs` | Add `load_spec_entry_with_diagnostics()`, `levenshtein()`, `find_fuzzy_match()` |
| `crates/assay-core/src/gate/mod.rs` | Add exit code 127/126 detection in result formatting |
| `crates/assay-core/src/config/mod.rs` | Improve TOML error formatting with `format_toml_error()` |
| `crates/assay-cli/src/commands/gate.rs` | Use enriched error display in `stream_criterion()` |
| `crates/assay-cli/src/commands/spec.rs` | Use `load_spec_entry_with_diagnostics()` |
| `crates/assay-mcp/src/server.rs` | Use `load_spec_entry_with_diagnostics()` in `load_spec_entry_mcp()` |

---

*Phase: 31-error-messages*
*Research completed: 2026-03-10*
