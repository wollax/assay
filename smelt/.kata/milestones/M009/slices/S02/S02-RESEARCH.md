# S02: README + example manifest documentation — Research

**Date:** 2026-03-24

## Summary

S02 is a pure documentation slice with no code changes. The deliverables are a workspace-level `README.md` and inline field-level comments on all 7 example TOML files. The project has comprehensive CLI help output (`--help` on all 6 subcommands), a detailed `PROJECT.md`, and the existing examples provide the structural truth for every manifest field. The main risk is factual accuracy — the README must match the actual CLI interface and manifest schema, not describe imagined features.

The recommended approach is: (1) write `README.md` using `--help` output and `PROJECT.md` as authoritative sources, (2) annotate each example TOML by cross-referencing the serde structs in `manifest.rs` and `config.rs` to ensure every documented field actually exists and has the correct type/default.

## Recommendation

Write README top-down: overview → install → quickstart → subcommand reference → example walkthrough → ecosystem. For examples, work file-by-file adding `#` comments above or inline to each field, explaining purpose, valid values, and defaults. Verify all examples still parse with `cargo run -- run <file> --dry-run` after adding comments (TOML `#` comments are safe but verify no accidental structural changes).

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| CLI flag documentation | `cargo run -- <cmd> --help` output | Authoritative source; don't manually transcribe flag names |
| Manifest field reference | `crates/smelt-core/src/manifest.rs` serde structs | `deny_unknown_fields` means the structs ARE the schema |
| Server config reference | `crates/smelt-cli/src/serve/config.rs` struct | Same — serde struct is the config schema |

## Existing Code and Patterns

- `crates/smelt-core/src/manifest.rs` — All manifest types: `JobManifest`, `JobMeta`, `Environment`, `CredentialConfig`, `SessionDef`, `MergeConfig`, `ComposeService`, `KubernetesConfig`. Each has doc comments. `deny_unknown_fields` on all.
- `crates/smelt-cli/src/serve/config.rs` — `ServerConfig` struct for `server.toml`. Fields: `queue_dir`, `max_concurrent`, `retry_attempts`, `retry_backoff_secs`, `server.host`, `server.port`, `workers[]`.
- `crates/smelt-core/src/lib.rs` — Crate-level doc comment with feature flag docs and usage example. Good model for README library section.
- `examples/server.toml` — Already well-commented (20 comment lines / 29 total). Minimal work needed.
- `examples/job-manifest-forge.toml` — Partially commented (16 comments / 55 lines). Needs field-level expansion.
- `examples/agent-manifest.toml` — Zero comments, 15 lines. Needs full annotation.
- `examples/job-manifest-k8s.toml` — Zero comments, 33 lines. Needs full annotation including `[kubernetes]` block fields.
- `examples/bad-manifest.toml` — 4 comments. Needs explanation of each intentional error.
- `.kata/PROJECT.md` — Living project context doc. Source of truth for the "what is Smelt" narrative.

## Constraints

- D125: M009 is documentation/cleanup only — zero behavior changes
- TOML `#` comments only — no structural changes to example files
- All 7 examples must remain parseable (or intentionally invalid for `bad-manifest.toml`)
- README audience is CLI users, not library embedders (`cargo doc` covers smelt-core API)
- No binary releases or homebrew — install is `cargo install --path .` only
- No per-crate READMEs (out of scope per S02-CONTEXT)

## Common Pitfalls

- **Documenting features that don't exist** — The README must match actual `--help` output exactly. Don't invent flags or describe planned features. Cross-reference every claim against the binary.
- **Example TOML structural breakage** — Adding a comment on the wrong line or accidentally removing a bracket can break TOML parsing. Run `--dry-run` on each example after editing.
- **Stale field names in examples** — The K8s example uses fields like `cpu_request`, `memory_limit` etc. Verify these match `KubernetesConfig` struct fields exactly.
- **Missing the `[manifest]` vs `[job]` distinction** — `agent-manifest.toml` uses `[manifest]` key while all other examples use `[job]`. This is an existing schema difference — document it, don't "fix" it.

## Open Risks

- **`agent-manifest.toml` uses `[manifest]` not `[job]`** — Need to verify whether this is a valid alternative key or an outdated example. Check `manifest.rs` for `#[serde(alias)]` or similar.
- **README length** — Covering 6 subcommands + serve HTTP API + TUI + SSH workers + server.toml could make the README very long. May need to keep sections concise and link to examples rather than duplicating content.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust / cargo doc | n/a | No agent skill needed — pure documentation writing |

## Sources

- `cargo run -- --help` and all subcommand `--help` — CLI interface truth
- `crates/smelt-core/src/manifest.rs` — Manifest schema truth (serde structs)
- `crates/smelt-cli/src/serve/config.rs` — Server config schema truth
- `.kata/PROJECT.md` — Project narrative and architecture overview
- `.kata/DECISIONS.md` — 126 architectural decisions providing context for README claims
