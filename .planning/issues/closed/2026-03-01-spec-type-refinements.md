> **Closed:** 2026-03-15 — Won't fix. Superseded by v0.4.0 architecture (phases 35-44).


---
title: "Spec type refinements from Phase 6 review"
area: assay-core, assay-types
priority: low
source: PR review #27
---

# Spec Type Refinements

## Problem

Type design findings from Phase 6 PR review:

1. **ScanResult.specs uses untyped tuple** — `Vec<(String, Spec)>` could be a named struct (e.g., `ScanEntry { slug: String, spec: Spec }`) for better self-documentation and future extensibility
2. **SpecError fields are pub** — `field` and `message` on `SpecError` are public, allowing external construction. Consider private fields with constructor/accessor pattern if stronger invariants are needed
3. **Io NotFound match arm uses fragile string matching** — CLI error handling for config-not-found uses `matches!(source.kind(), std::io::ErrorKind::NotFound)` which is correct, but the surrounding pattern match could be more robust

Additional findings from second review:

4. **Duplicate error doesn't report which file "won"** — error says `duplicate spec name 'X'` but doesn't tell user which file holds the surviving copy. Should include `(also defined in first.toml)`.
5. **`file_stem` failure reported as `AssayError::Io`** — category mismatch: non-I/O failure wrapped in Io variant with fabricated `std::io::Error`. Consider dedicated `InvalidPath` variant.
6. **`Criterion.description` has no emptiness constraint** — `validate()` rejects empty `name` but allows empty `description`. Inconsistency should be either enforced or documented as intentional.
7. **ScanResult.errors conflates I/O and semantic errors** — parse failures and duplicate-name rejections share one `Vec<AssayError>`, requiring callers to pattern-match to distinguish categories.
8. **Stale process comment in AssayError doc** — "New variants are added as downstream phases consume them" is roadmap language that won't age well.
9. **Criterion `prompt` comment uses definite future tense** — "a future `prompt` field will support" should be softened to "may" since it's speculative.
10. **Spec.name doc should clarify where uniqueness is enforced** — says "must be unique across all specs" but uniqueness is only enforced at `scan()` level, not in `validate()`.

## Solution

- Consider introducing `ScanEntry` named struct (breaking change, defer to next major phase boundary)
- Evaluate whether SpecError invariants warrant encapsulation (currently low risk since it's crate-internal)
- Review Io error matching patterns for robustness
- Include surviving file path in duplicate error messages
- Evaluate dedicated `InvalidPath` error variant
- Decide on criterion description validation policy and document
- Consider splitting ScanResult.errors by category
- Clean up process/speculative comments