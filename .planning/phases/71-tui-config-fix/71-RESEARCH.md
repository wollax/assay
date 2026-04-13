# Phase 71: TUI Config Fix - Research

**Researched:** 2026-04-13
**Domain:** Rust TUI (ratatui) — config-aware path resolution in assay-tui
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Config fallback:**
- When `self.config` is `None` (no config.toml), fall back to default `"specs/"` silently — same behavior as CLI
- No warning or error needed for missing config; the default value already matches historical behavior

**slash.rs config threading:**
- Pass resolved `specs_dir: &Path` into `execute_slash_cmd` rather than the full `Config` struct
- Keeps function signature minimal — caller (app.rs) resolves specs_dir from config before calling
- Matches the principle of resolving config values at the surface boundary, not deep in dispatch

### Claude's Discretion
- Whether to extract a helper method for specs_dir resolution on App or inline it at each call site
- Test approach for verifying the fix (existing test patterns in gate_wizard_app.rs and slash_commands.rs)

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope
</user_constraints>

---

## Summary

Phase 71 is a mechanical bug fix: the TUI hardcodes `.assay/specs` (via `assay_dir.join("specs")`) at every call site that resolves specs, while the CLI consistently uses `assay_dir.join(&config.specs_dir)`. This means projects with a custom `specs_dir` config value silently fall back to the wrong directory when using the TUI.

The fix involves two files: `app.rs` (10 hardcoded sites) and `slash.rs` (2 hardcoded sites). The CLI pattern is already proven and well-established. The `Config` struct and `default_specs_dir()` already exist in `assay-types`; `self.config: Option<assay_types::Config>` is already loaded on `App` at startup. No new types, no new APIs — purely wiring existing values to existing call sites.

`execute_slash_cmd` in slash.rs must receive a `specs_dir: &Path` parameter because it currently constructs `specs_dir` internally from a hardcoded `"specs"` segment. The caller (app.rs) will resolve `specs_dir` from config before calling.

**Primary recommendation:** Add a private `App::resolved_specs_dir()` helper that reads `self.config.as_ref().map(|c| &c.specs_dir).map(String::as_str).unwrap_or("specs/")` and joins with `assay_dir`, then replace all 10 inline hardcodes with this helper. Update `execute_slash_cmd` signature to accept `specs_dir: &Path`.

## Standard Stack

### Core (no new dependencies)
| Component | Current Version | Purpose |
|-----------|----------------|---------|
| `assay-types::Config` | workspace | Holds `specs_dir: String` with `default_specs_dir()` fallback |
| `assay-core::config::load` | workspace | Loads config.toml — already called in `App::with_project_root` |
| `std::path::Path` | stdlib | Parameter type for `specs_dir` in updated signatures |

**Installation:** No new dependencies. This is pure code change within existing workspace.

## Architecture Patterns

### Canonical CLI Pattern (proven, copy verbatim)

The CLI resolves `specs_dir` exactly like this at every command handler:

```rust
// Source: crates/assay-cli/src/commands/gate.rs (multiple sites)
let specs_dir = assay_dir.join(&config.specs_dir);
```

When no config is available, the CLI falls back via `Config::default()` (which calls `default_specs_dir()` returning `"specs/"`).

### TUI Resolution Pattern (to implement)

The TUI stores config as `Option<Config>`. The resolution pattern follows the locked decision (fallback to default silently):

```rust
// Proposed private helper on App
fn resolved_specs_dir(&self, assay_dir: &std::path::Path) -> std::path::PathBuf {
    let rel = self
        .config
        .as_ref()
        .map(|c| c.specs_dir.as_str())
        .unwrap_or("specs/");
    assay_dir.join(rel)
}
```

This is called wherever `assay_dir.join("specs")` currently appears in app.rs.

### Updated execute_slash_cmd Signature

Current:
```rust
pub fn execute_slash_cmd(cmd: SlashCmd, project_root: &Path) -> String {
    let assay_dir = project_root.join(".assay");
    let specs_dir = assay_dir.join("specs");   // hardcoded
    ...
}
```

Updated (locked decision: caller resolves specs_dir):
```rust
pub fn execute_slash_cmd(cmd: SlashCmd, project_root: &Path, specs_dir: &Path) -> String {
    let assay_dir = project_root.join(".assay");
    // specs_dir is now a parameter — no local construction
    ...
}
```

Call site in app.rs (the `_ =>` fallthrough for non-screen-transition slash commands):
```rust
let result = execute_slash_cmd(cmd, root, &self.resolved_specs_dir(&root.join(".assay")));
```

### Complete Inventory of Hardcoded Sites in app.rs

Confirmed by grep — 10 occurrences of `assay_dir.join("specs")`:

| Line | Context |
|------|---------|
| 436 | `GateWizardAction::Submit` — writes gate to disk |
| 924 | `/gate-wizard` slash cmd — collects available_gates |
| 944 | `/gate-edit` slash cmd — loads spec for edit |
| 952 | `/gate-edit` slash cmd — collects available_gates after load |
| 1046 | `'g'` key on Dashboard — collects available_gates |
| 1287 | `Enter` in MilestoneDetail — loads spec on navigation |
| 1345 | `'e'` key in ChunkDetail — collects available_gates for edit |
| 1350 | `'e'` key in ChunkDetail — loads spec entry |
| 1352 | `'e'` key in ChunkDetail — collects available_gates |
| 1395 | `WizardAction::Submit` — creates milestone+spec |

### Hardcoded Sites in slash.rs

`execute_slash_cmd` constructs `specs_dir` at line 140:
```rust
let specs_dir = assay_dir.join("specs");   // line 140 — hardcoded
```

This is the only definition; it is used at lines 181 (`NextChunk`), 207 (`GateCheck`/`pr_check_milestone_gates`), 239 (`SpecShow`), and 261 (`PrCreate`/`pr_check_milestone_gates`).

### Anti-Patterns to Avoid

- **Do not pass `Option<&Config>` into `execute_slash_cmd`** — locked decision says pass resolved `&Path`, not full config struct
- **Do not resolve specs_dir inside each match arm** — put the resolution in the helper or at the top of the method, not duplicated inline
- **Do not change test fixture paths** — existing tests use `.assay/specs/` which matches `default_specs_dir()`, so they continue to pass without modification

## Don't Hand-Roll

| Problem | Don't Build | Use Instead |
|---------|-------------|-------------|
| Default specs dir string | Custom constant `"specs"` | `assay_types::default_specs_dir()` indirectly via `Config::default()` or fallback to `"specs/"` literal matching the function return value |
| Config loading | Any new load logic | `assay_core::config::load(root)` — already in use in `App::with_project_root` |

## Common Pitfalls

### Pitfall 1: Inconsistent fallback string
**What goes wrong:** Using `"specs"` instead of `"specs/"` as the fallback. `default_specs_dir()` returns `"specs/"` (with trailing slash). `assay_dir.join("specs/")` and `assay_dir.join("specs")` resolve identically on both Unix and Windows (Rust's `Path::join` handles it), so functionally this doesn't matter. However, using `"specs/"` is consistent with the type definition.

**How to avoid:** Use `"specs/"` in the fallback to match `default_specs_dir()`.

### Pitfall 2: Missing the `WizardAction::Submit` site
**What goes wrong:** The Wizard (milestone creation wizard, not gate wizard) at line 1395 also has a hardcoded `assay_dir.join("specs")` and passes `specs_dir` to `create_from_inputs`. Easy to miss because it's in the `Screen::Wizard` arm, not `Screen::GateWizard`.

**How to avoid:** Use the helper method for all 10 sites, not just the gate wizard sites.

### Pitfall 3: execute_slash_cmd callers not updated
**What goes wrong:** Updating the function signature without updating all callers. There is one call site in app.rs (line 986: `execute_slash_cmd(cmd, root)`) and multiple test usages.

**How to avoid:** After updating the signature, check `cargo check -p assay-tui` immediately — the compiler will catch all missed callers. Also update tests in `slash_commands.rs` (no direct calls to `execute_slash_cmd` in that test file — tests go through `App::handle_event` — so test changes may be minimal).

### Pitfall 4: Tests construct fixtures in `.assay/specs/`
**What goes wrong:** Worrying that test fixture setup needs to change. It does not: all existing test helpers write to `assay_dir.join("specs")`, which exactly matches `default_specs_dir()`. Since no test sets a custom `specs_dir` in config.toml, the behavior is identical before and after the fix.

**How to avoid:** Do not modify existing test fixtures. Only add a new test (if desired) that verifies custom `specs_dir` config is honored.

## Code Examples

### Existing App config loading (already correct)

```rust
// Source: crates/assay-tui/src/app.rs lines 255-270
let config = project_root.as_deref().and_then(|root| {
    let config_path = root.join(".assay").join("config.toml");
    if config_path.exists() {
        match assay_core::config::load(root) {
            Ok(cfg) => Some(cfg),
            Err(e) => {
                tracing::warn!(error = %e, "Failed to load .assay/config.toml");
                None
            }
        }
    } else {
        None
    }
});
```

Config is stored as `pub config: Option<assay_types::Config>` on App (line 192).

### Confirmed default_specs_dir value

```rust
// Source: crates/assay-types/src/lib.rs lines 430-432
fn default_specs_dir() -> String {
    "specs/".to_string()
}
```

### Config.specs_dir field declaration

```rust
// Source: crates/assay-types/src/lib.rs lines 208-209
#[serde(default = "default_specs_dir")]
pub specs_dir: String,
```

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test + integration test files in `tests/` |
| Config file | `crates/assay-tui/` (no separate test config; uses workspace Cargo.toml) |
| Quick run command | `cargo test -p assay-tui 2>&1` |
| Full suite command | `just test` |

### Phase Requirements → Test Map

No formal requirement IDs for this phase (gap-closure fix). Behavioral requirements:

| Behavior | Test Type | Automated Command |
|----------|-----------|-------------------|
| TUI resolves specs from `config.specs_dir` | Integration | New test in `gate_wizard_app.rs` or `slash_commands.rs` — write a fixture with custom `specs_dir` in config.toml, verify gate is written to custom path |
| Fallback when no config.toml | Regression | Existing tests pass without modification (all use default path) |
| `/gate-wizard` slash cmd uses config path | Integration | Covered by gate_wizard_app.rs or slash_commands.rs if custom-dir test added |

### Sampling Rate
- **Per task commit:** `cargo test -p assay-tui`
- **Per wave merge:** `just ready`
- **Phase gate:** `just ready` green before `/kata:verify-work`

### Wave 0 Gaps

- [ ] New integration test verifying custom `specs_dir` honored in gate wizard write path (covers success criterion 2)

Existing infrastructure covers regression testing of the default path (all 14 test files already pass).

## Sources

### Primary (HIGH confidence)

- Direct code inspection: `crates/assay-tui/src/app.rs` — all 10 hardcoded sites confirmed by grep
- Direct code inspection: `crates/assay-tui/src/slash.rs` — hardcoded `join("specs")` at line 140 confirmed
- Direct code inspection: `crates/assay-types/src/lib.rs` — `Config.specs_dir`, `default_specs_dir()` confirmed
- Direct code inspection: `crates/assay-cli/src/commands/gate.rs` — canonical `assay_dir.join(&config.specs_dir)` pattern confirmed at multiple sites

### Secondary (MEDIUM confidence)

- CONTEXT.md decisions (from /kata:discuss-phase) — implementation strategy is locked and authoritative

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies, all types and functions verified by code inspection
- Architecture: HIGH — all 10 app.rs sites and 2 slash.rs sites confirmed by grep; CLI pattern confirmed
- Pitfalls: HIGH — identified from direct code reading, not speculation

**Research date:** 2026-04-13
**Valid until:** N/A — this is internal code, not an external dependency; findings are stable until the files change
