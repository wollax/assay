# Decisions Register

<!-- Append-only. Never edit or remove existing rows.
     To reverse a decision, add a new row that supersedes it.
     Read this file at the start of any planning or research phase. -->

| # | When | Scope | Decision | Choice | Rationale | Revisable? |
|---|------|-------|----------|--------|-----------|------------|
| D001 | M001 | arch | Control inversion pattern | Closures/callbacks, not traits | Zero-trait codebase convention (0 traits across 17K lines); all 3 brainstorm pairs converged | No |
| D002 | M001 | arch | Orchestration location | `assay-core::orchestrate` module (feature-gated) | Module avoids premature crate boundary; feature gate provides rollback safety | Yes — if module grows past ~3K lines |
| D003 | M001 | arch | Harness adapter location | New `assay-harness` leaf crate | Implementations depend on core, not vice versa; preserves clean dep graph | No |
| D004 | M001 | convention | Manifest format | TOML with `[[sessions]]` array | Forward-compatible for multi-agent; `[[sessions]]` even for single-session avoids breaking change | No |
| D005 | M001 | convention | MCP tool evolution | Additive only — new namespaced tools, never modify existing signatures | Preserves backward compatibility for existing 18 tool consumers | No |
| D006 | M001 | arch | Session vocabulary | AgentSession → GateEvalContext; Smelt manifest → RunManifest; runner → RunExecutor | 5 "session" concepts cause confusion; clean names before adding more types | No |
| D007 | M001 | arch | Launcher sync vs async | Sync (std::process::Command + spawn_blocking) | Single-shot launch is request-response; async adds complexity without benefit until M002 | Yes — if M002 needs async |
| D008 | M001 | arch | Git interaction | Shell out to git CLI, no git2/gix | git2 is !Send+!Sync (breaks spawn_blocking), gix has 150+ transitive deps, git CLI is proven | No |
| D009 | M001 | arch | Session persistence | JSON file-per-record, not SQLite | Single-project scope (tens-to-hundreds of sessions); consistent with existing history module | Yes — if scale exceeds thousands |
| D010 | M001 | arch | HarnessProfile location | Type in assay-types, implementation in assay-harness | Cross-crate via serialization; follows existing pattern for types vs logic separation | No |
| D011 | M001/S03 | convention | Settings merger field construction | Explicit struct construction (no `..base`) | Compile-time safety — adding a field to SettingsOverride forces merge_settings to handle it | No |
| D012 | M001/S03 | convention | Vec merge semantics | Replace (non-empty override wins entirely) | Extend semantics would cause surprising duplicates; replace is predictable and explicit | Yes — if extend is needed later |
