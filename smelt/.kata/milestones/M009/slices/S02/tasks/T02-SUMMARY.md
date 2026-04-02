---
id: T02
parent: S02
milestone: M009
provides:
  - Fixed agent-manifest.toml ([manifest]→[job] + all required sections)
  - Field-level comments on all 7 example TOML files
  - bad-manifest.toml documents all 7 validation violations inline
  - All comments cross-referenced against manifest.rs/config.rs structs
  - Comment coverage: 22-47 lines per file (all well above minimums)
key_files:
  - examples/agent-manifest.toml
  - examples/bad-manifest.toml
  - examples/job-manifest.toml
  - examples/job-manifest-compose.toml
  - examples/job-manifest-forge.toml
  - examples/job-manifest-k8s.toml
  - examples/server.toml
key_decisions:
  - "agent-manifest.toml rewritten with all required sections (environment, credentials, merge) — the original was not just using [manifest] but also had invalid session fields (task/file_scope/timeout_secs instead of spec/harness/timeout)"
patterns_established:
  - "Example file comment style: header block explaining purpose + run command, then inline comments above or beside each field"
observability_surfaces:
  - none
duration: 15min
verification_result: passed
completed_at: 2026-03-24T19:54:00Z
blocker_discovered: false
---

# T02: Annotate all 7 example manifests with field-level comments

**Fixed broken agent-manifest.toml and added comprehensive inline comments to all 7 example files, cross-referenced against serde struct definitions**

## What Happened

Fixed `agent-manifest.toml` which had two problems: (1) used `[manifest]` instead of `[job]` (fails `deny_unknown_fields`), and (2) used invalid session fields (`task`, `file_scope`, `timeout_secs` instead of `spec`, `harness`, `timeout`). Rewrote it as a complete minimal manifest with all required sections.

Added field-level `#` comments to all 7 example files. Every field has an inline comment explaining its purpose, valid values, and defaults where applicable. All field names verified against the serde structs: `JobManifest`, `JobMeta`, `Environment`, `CredentialConfig`, `SessionDef`, `MergeConfig`, `ComposeService`, `KubernetesConfig`, `ForgeConfig` in `manifest.rs`/`forge.rs`, and `ServerConfig`/`ServerNetworkConfig`/`WorkerConfig` in `config.rs`.

For `bad-manifest.toml`, each intentional validation error is documented with a `# VIOLATION:` comment explaining which rule it breaks. The 7 violations: empty job.name, empty image, zero timeout, duplicate session name, unknown depends_on reference, empty merge.target, unknown merge.order reference.

## Verification

- All 6 valid examples pass `cargo run -- run examples/<file>.toml --dry-run` (exit 0)
- `bad-manifest.toml` exits non-zero with 7 validation errors (all still detected correctly)
- Comment line counts: agent=28, bad=22, compose=46, forge=44, k8s=47, job=41, server=41 (all exceed minimums)
- `cargo test --workspace` — 155 tests pass, 0 failures

### Slice Verification (S02)
| Check | Status |
|-------|--------|
| README.md exists, 200+ lines | ✓ PASS (T01) |
| job-manifest.toml --dry-run exits 0 | ✓ PASS |
| job-manifest-forge.toml --dry-run exits 0 | ✓ PASS |
| job-manifest-compose.toml --dry-run exits 0 | ✓ PASS |
| job-manifest-k8s.toml --dry-run exits 0 | ✓ PASS |
| agent-manifest.toml --dry-run exits 0 | ✓ PASS |
| bad-manifest.toml --dry-run exits non-zero | ✓ PASS |
| agent-manifest.toml ≥5 comment lines | ✓ PASS (28) |
| job-manifest-k8s.toml ≥10 comment lines | ✓ PASS (47) |
| cargo test --workspace passes | ✓ PASS (155 tests) |

All slice verification checks pass.

## Diagnostics

None — pure documentation artifacts. Inspect by reading example files directly.

## Deviations

agent-manifest.toml required more extensive rewriting than planned. Beyond the `[manifest]→[job]` fix, the original also used invalid session fields (`task`, `file_scope`, `timeout_secs`) that don't exist in `SessionDef`. The file was rewritten as a complete valid manifest with all required sections (environment, credentials, merge) that were missing.

## Known Issues

None.

## Files Created/Modified

- `examples/agent-manifest.toml` — Rewritten: [manifest]→[job], invalid fields fixed, all required sections added, full comments
- `examples/bad-manifest.toml` — Added VIOLATION comments documenting each of the 7 validation errors
- `examples/job-manifest.toml` — Expanded to full field-level comments on every field
- `examples/job-manifest-compose.toml` — Expanded comments including [[services]] passthrough explanation
- `examples/job-manifest-forge.toml` — Expanded comments including [forge] section documentation
- `examples/job-manifest-k8s.toml` — Added header and full field-level comments on all [kubernetes] fields
- `examples/server.toml` — Expanded comments with [[workers]] field documentation
