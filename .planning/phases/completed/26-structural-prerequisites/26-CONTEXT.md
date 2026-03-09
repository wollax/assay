# Phase 26: Structural Prerequisites - Context

**Gathered:** 2026-03-09
**Status:** Ready for planning

<domain>
## Phase Boundary

Extract the 2563-line CLI monolith (`main.rs`) into per-subcommand modules, add a `serde_json` error variant to `AssayError` with ergonomic constructors, and verify the existing TUI→core dependency works. This unblocks all subsequent v0.3.0 feature and hardening work.

</domain>

<decisions>
## Implementation Decisions

### CLI module boundaries
- Split CLI into `commands/` modules — one module per subcommand group (mcp, spec, gate, context, checkpoint, guard)
- `main.rs` retains `Cli` struct, top-level `Command` enum, and dispatch logic only

### Claude's Discretion
- Whether `init` gets its own module or stays inline in `main.rs` (it's a single command with minimal logic)
- Flat files (`commands/gate.rs`) vs nested directories (`commands/gate/mod.rs`) — pick based on current complexity of each group
- Whether subcommand enums (`GateCommand`, `SpecCommand`, etc.) live in `main.rs` or move into their respective modules
- Where shared CLI helpers live (color output, ANSI constants, formatting) — `commands/mod.rs` vs dedicated `helpers.rs`/`output.rs`

### Error variant granularity
- Add a `serde_json`-specific error variant to distinguish JSON serialization/deserialization failures from I/O errors
- Add ergonomic constructor helpers (e.g., `AssayError::io(...)`, `AssayError::json(...)`) to reduce verbose struct construction

### Claude's Discretion (Errors)
- Whether to introduce domain sub-enums (GuardError, SessionError, etc.) now or keep the flat enum with just the new JSON variant
- Inherent methods on `AssayError` vs module-level free functions for constructors
- `impl Into<String>` vs `&str` for operation/message parameters on helpers

### TUI-core wiring scope
- `assay-tui` already has `assay-core` in `Cargo.toml` — verify the import works; no major TUI functional changes required this phase

### Claude's Discretion (TUI)
- Whether to go beyond verify-only and start importing specific core types (spec/gate types for a real TUI view)
- Whether to keep `color-eyre` for TUI error handling or begin aligning with `AssayError`

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. The user delegated all implementation style decisions to Claude, indicating trust in idiomatic Rust patterns and pragmatic engineering judgment.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 26-structural-prerequisites*
*Context gathered: 2026-03-09*
