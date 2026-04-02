# S02: README + example manifest documentation

**Goal:** Comprehensive workspace-level README and fully annotated example manifests so new users can understand, install, and use Smelt without reading source code.
**Demo:** `README.md` exists at workspace root with project overview, install instructions, all 6 subcommand usage sections, and an example walkthrough; all 7 example manifests have inline field-level comments; all examples that should parse still parse after annotation.

## Must-Haves

- `README.md` at workspace root covers: what Smelt is, install, quickstart, all 6 subcommands (init, list, run, serve, status, watch), example walkthrough, ecosystem (Assay/Cupel)
- All 7 example TOML files have inline `#` comments explaining every field's purpose, valid values, and defaults
- `agent-manifest.toml` fixed to use `[job]` instead of invalid `[manifest]` key (currently fails parsing — D017 `deny_unknown_fields`)
- All parseable examples still pass `smelt run <file> --dry-run` after annotation
- `bad-manifest.toml` documents each intentional error with a comment

## Proof Level

- This slice proves: contract (documentation correctness verified against CLI and schema)
- Real runtime required: no (dry-run only)
- Human/UAT required: yes (README readability is subjective; UAT script provided)

## Verification

- `test -f README.md && wc -l README.md` — file exists, substantial length (200+ lines)
- `cargo run -- run examples/job-manifest.toml --dry-run` — exits 0
- `cargo run -- run examples/job-manifest-forge.toml --dry-run` — exits 0
- `cargo run -- run examples/job-manifest-compose.toml --dry-run` — exits 0
- `cargo run -- run examples/job-manifest-k8s.toml --dry-run` — exits 0
- `cargo run -- run examples/agent-manifest.toml --dry-run` — exits 0 (currently fails; fix `[manifest]` → `[job]`)
- `cargo run -- run examples/bad-manifest.toml --dry-run` — exits non-zero (intentionally invalid)
- `grep -c '^#' examples/agent-manifest.toml` — at least 5 comment lines
- `grep -c '^#' examples/job-manifest-k8s.toml` — at least 10 comment lines
- `cargo test --workspace` — all 286+ tests pass (no regressions)

## Observability / Diagnostics

- Runtime signals: none (pure documentation slice)
- Inspection surfaces: none
- Failure visibility: none
- Redaction constraints: none

## Integration Closure

- Upstream surfaces consumed: `--help` output from all 6 subcommands; `manifest.rs` serde structs; `config.rs` ServerConfig struct; `.kata/PROJECT.md` for narrative
- New wiring introduced in this slice: none (documentation only)
- What remains before the milestone is truly usable end-to-end: S01 (cargo doc zero-warning, deny(missing_docs) on smelt-cli — already complete), S03 (large file decomposition)

## Tasks

- [x] **T01: Write workspace README.md** `est:45m`
  - Why: R041 — no README exists; new users have zero entry point
  - Files: `README.md`
  - Do: Write comprehensive README using `--help` output and PROJECT.md as authoritative sources. Sections: overview, install, quickstart (smelt init → smelt run --dry-run), subcommand reference (all 6), server mode overview, example walkthrough, ecosystem table. Cross-reference every flag/option against actual `--help` output. Keep concise — link to examples rather than duplicating manifest content.
  - Verify: `test -f README.md && wc -l README.md` shows 200+ lines; all command names and flags match `--help` output
  - Done when: README covers all 6 subcommands with accurate flags, install instructions, and example walkthrough

- [x] **T02: Annotate all 7 example manifests with field-level comments** `est:30m`
  - Why: R045 — uncommented examples force users to read source code; agent-manifest.toml is currently broken (uses `[manifest]` instead of `[job]`)
  - Files: `examples/agent-manifest.toml`, `examples/bad-manifest.toml`, `examples/job-manifest.toml`, `examples/job-manifest-compose.toml`, `examples/job-manifest-forge.toml`, `examples/job-manifest-k8s.toml`, `examples/server.toml`
  - Do: Fix `agent-manifest.toml` `[manifest]` → `[job]`. Add/expand `#` comments on every field in all 7 files. Cross-reference field names against `manifest.rs` structs and `config.rs` `ServerConfig`. For `bad-manifest.toml`, document each intentional error. For `server.toml`, expand existing comments. Verify all parseable examples with `--dry-run` after editing.
  - Verify: All 6 valid examples pass `--dry-run`; `bad-manifest.toml` fails as expected; each example has substantial comment coverage
  - Done when: Every field in every example has an inline comment; all parseable examples verified with `--dry-run`

## Files Likely Touched

- `README.md`
- `examples/agent-manifest.toml`
- `examples/bad-manifest.toml`
- `examples/job-manifest.toml`
- `examples/job-manifest-compose.toml`
- `examples/job-manifest-forge.toml`
- `examples/job-manifest-k8s.toml`
- `examples/server.toml`
