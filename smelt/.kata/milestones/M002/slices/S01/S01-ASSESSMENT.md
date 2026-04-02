---
id: S01-ASSESSMENT
slice: S01
milestone: M002
assessed_at: 2026-03-17
verdict: roadmap_unchanged
---

# Roadmap Assessment After S01

## Verdict

Roadmap is unchanged. S01 retired its assigned risk and produced every deliverable in the S01→S02 boundary map. Remaining slices S02–S04 are still correct as written.

## Risk Retirement

S01 was assigned the "AssayInvoker contract" risk — the highest-priority risk in M002. It retired it completely: four `deny_unknown_fields` serde types replace the broken `AssayManifest`/`AssaySession` types, 13 unit tests prove all contract invariants (`[[sessions]]` key, spec-name references, no `harness`/`timeout` fields, sanitized names, `--base-branch` flag), and `cargo test --workspace` exits 0 with 110 smelt-core tests passing.

No new risks or unknowns emerged that change slice ordering or scope.

## Success Criteria Coverage

- `AssayInvoker generates RunManifest ([[sessions]] plural, spec=name reference) + per-session Spec TOML` → **S01 ✅ (TOML contract)**, S02 (real binary acceptance)
- `smelt run manifest.toml runs full pipeline` → S02, S03, S04
- `Gate output visible on terminal as assay run produces it (streaming)` → S03
- `assay run exit code 2 surfaced as distinct outcome` → S04
- `run_without_dry_run_attempts_docker pre-existing failure resolved` → S02

All five success criteria have at least one remaining owning slice. Coverage is sound.

## Boundary Map Accuracy

S01→S02 boundary map is accurate against what was built:
- All eight listed `AssayInvoker` methods exist in `assay.rs`
- `write_spec_file_to_container` is async, takes `&dyn RuntimeProvider`, and mirrors `write_manifest_to_container` — exactly what S02 Phase 5.5 wiring expects
- Sanitized name agreement (manifest session `spec` field ↔ spec filename) is guaranteed by both calling `sanitize_session_name(&session.name)`
- D043 appended to `DECISIONS.md`, superseding D029

## One Fragile Point to Watch in S02

`build_write_assay_config_command` and `write_spec_file_to_container` both require `base64` in the container image. Alpine has it by default, but if S02 tests use a non-Alpine image this will fail silently (exec exits non-zero, Err path triggers). Not a plan change — just worth verifying the container image in the integration test is Alpine-based.

## Requirements

No `REQUIREMENTS.md` exists — operating in legacy compatibility mode. No requirement coverage changes.
