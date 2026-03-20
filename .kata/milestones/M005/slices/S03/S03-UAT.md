# S03: Guided Authoring Wizard — UAT

**Milestone:** M005
**Written:** 2026-03-20

## UAT Type

- UAT mode: mixed (artifact-driven for MCP/core; human-experience for interactive CLI)
- Why this mode is sufficient: The wizard core and MCP tools are fully automated — integration tests prove the generated files are valid and parseable. The TTY path (`assay plan` dialoguer prompts) requires a human in an interactive terminal because automated tests always hit the non-TTY guard; a quick manual run confirms prompt rendering and UX.

## Preconditions

For automated (artifact-driven) portion:
- `just ready` is green
- In a temp project: `mkdir .assay/milestones .assay/specs`

For manual (human-experience) portion:
- Real terminal (not CI, not piped stdin)
- `assay` binary built: `cargo build -p assay-cli`
- Empty test project directory: `mkdir /tmp/uat-assay && cd /tmp/uat-assay && mkdir -p .assay/milestones .assay/specs`

## Smoke Test

Run the non-TTY guard in a subshell to confirm exit behavior:

```bash
echo "" | cargo run -p assay-cli -- plan 2>&1
# Expected: message mentioning "milestone_create MCP tool"; exit code 1
```

## Test Cases

### 1. Wizard core round-trip (artifact-driven)

The integration test `wizard_create_from_inputs_writes_files` covers this automatically. To verify manually:

1. Run `cargo test -p assay-core --features assay-types/orchestrate --test wizard`
2. **Expected:** 5 tests pass; milestone TOML + two `gates.toml` files created in temp dirs and reloaded cleanly

### 2. Slug collision rejection

1. Run `cargo test -p assay-core --features assay-types/orchestrate --test wizard wizard_slug_collision`
2. **Expected:** `wizard_slug_collision_returns_error` passes — second `create_from_inputs` with same slug returns `Err`

### 3. Spec patches milestone

1. Run `cargo test -p assay-core --features assay-types/orchestrate --test wizard wizard_create_spec_patches`
2. **Expected:** `wizard_create_spec_patches_milestone` passes — milestone's `chunks` vec contains the new chunk slug after `create_spec_from_params`

### 4. MCP milestone_create end-to-end

1. Run `cargo test -p assay-mcp -- milestone_create`
2. **Expected:** 3 tests pass (`milestone_create_tool_in_router`, `milestone_create_writes_milestone_toml`, plus router presence)
3. On success, the `milestone_create_writes_milestone_toml` test proves `.assay/milestones/test-ms.toml` is created and the response is the slug string `"test-ms"`

### 5. MCP spec_create with duplicate rejection

1. Run `cargo test -p assay-mcp -- spec_create`
2. **Expected:** 3 tests pass; `spec_create_rejects_duplicate` confirms `isError: true` on second call with same slug

### 6. assay plan non-TTY guard (artifact-driven)

1. Run `cargo test -p assay-cli -- plan`
2. **Expected:** `plan_non_tty_returns_1` passes (exit code 1 returned)

### 7. assay plan interactive — manual UAT

1. `cd /tmp/uat-assay && mkdir -p .assay/milestones .assay/specs`
2. Run `path/to/assay plan`
3. At "Milestone name" prompt, enter: `My Feature`
4. At "Description" prompt, enter: `A test milestone`
5. At "Number of chunks" select: `2`
6. At chunk 1 name prompt, enter: `Auth module`
7. At "Add a criterion?" confirm: yes
8. At criterion description, enter: `Login endpoint returns 200`
9. At "Add another criterion?" confirm: no
10. At chunk 2 name prompt, enter: `Profile page`
11. At "Add a criterion?" confirm: yes, enter `Profile renders user data`, then no more
12. **Expected:** Two files printed — `.assay/milestones/my-feature.toml` and two `gates.toml` files under `.assay/specs/auth-module/` and `.assay/specs/profile-page/`

### 8. Generated files validate (manual, follows case 7)

1. Run `assay milestone list`
2. **Expected:** `my-feature` milestone appears with status `draft` and 2 chunks
3. Run `assay spec list`
4. **Expected:** `auth-module` and `profile-page` specs visible
5. Run `assay gate run auth-module`
6. **Expected:** Gate runs (may fail criteria — that's fine), but no parse errors

## Edge Cases

### Non-existent milestone_slug in spec_create

1. Run `cargo test -p assay-core --features assay-types/orchestrate --test wizard wizard_create_spec_rejects_nonexistent`
2. **Expected:** `wizard_create_spec_rejects_nonexistent_milestone` passes — returns `Err` when `milestone_slug = Some("ghost")` with no backing file

### Criteria-only gates (no cmd)

When `spec_create` MCP tool is called with `criteria: ["some description"]`:
1. The generated `gates.toml` has a criterion with `description` but no `cmd` field
2. **Expected:** File is valid TOML and parses without error; gate run will fail the criterion (no command to execute), but won't crash

## Failure Signals

- `cargo test -p assay-core --test wizard` fails → wizard core broken; check `crates/assay-core/src/wizard.rs`
- `cargo test -p assay-mcp -- milestone_create spec_create` fails → MCP param structs or tool methods broken; check `server.rs`
- `assay plan` hangs without output in a terminal → dialoguer TTY detection or Select/Input issue
- Generated `gates.toml` fails to parse → `write_gates_toml` template format broken
- `assay milestone list` shows no results after wizard run → `milestone_save` path resolution wrong; check `assay_dir` passed to `create_from_inputs`

## Requirements Proved By This UAT

- R042 (Guided authoring wizard) — integration tests prove wizard core produces valid, parseable milestone TOML + gates.toml files; MCP tools provide programmatic parity; CLI non-TTY guard proven automated; interactive TTY path proven by manual case 7–8

## Not Proven By This UAT

- Full `assay plan` dialoguer rendering on non-macOS terminals (Linux, Windows WSL) — manual UAT above covers macOS only
- `assay plan` behavior when the user presses Ctrl-C mid-wizard — partial writes are not currently cleaned up
- Criteria with shell commands verified as runnable — `spec_create` accepts `Vec<String>` descriptions only; cmd fields not collected by wizard; runnable gates require manual editing
- PR creation from a wizard-generated milestone (S04 scope)
- Claude Code `/assay:plan` skill calling `milestone_create` (S05 scope)

## Notes for Tester

- The interactive case (7–8) only works in a real TTY; running from a script or pipe will hit the non-TTY guard immediately
- Generated spec slugs are lowercase-hyphenated versions of chunk names via `slugify`; verify the slug matches your input
- Gates generated from wizard criteria will have no `cmd` field — `assay gate run` will mark them as failing (expected) until commands are added manually
- `assay plan` prints created file paths at the end; cross-check these against the actual filesystem
