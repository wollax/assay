# S04: Wizard runnable criteria — UAT

**Milestone:** M013
**Written:** 2026-03-28

## UAT Type

- UAT mode: mixed (artifact-driven contract tests + human-experience for interactive TTY/TUI surfaces)
- Why this mode is sufficient: The cmd field round-trip (wizard input → gates.toml → gate run) is fully proven by contract tests with real filesystem I/O. The interactive CLI wizard TTY path and TUI form require a human operator because automated tests cannot drive a real TTY prompt loop or verify visual prompt rendering.

## Preconditions

- `just build` passes (workspace compiles clean)
- An assay project with `.assay/milestones/` and `.assay/specs/` directories (or `assay init` in a temp dir)
- Terminal with TTY support (not piped — wizard exits non-zero on non-TTY input)

## Smoke Test

Run `assay plan` in a project dir, enter a milestone goal, chunk count, chunk name, one criterion name, and a command. Confirm the generated `gates.toml` contains a `cmd` field. Then run `gate run` — it should succeed immediately without manual editing.

## Test Cases

### 1. CLI wizard: criterion with cmd

1. `cd <project-dir> && assay plan`
2. Enter milestone goal: "Test milestone"
3. Enter number of chunks: 1
4. Enter chunk name: "my-chunk"
5. Enter criterion name: "tests pass"
6. Enter command: `cargo test`
7. Press Enter on blank criterion name to finish
8. Confirm: `cat .assay/specs/my-chunk/gates.toml`
9. **Expected:** File contains `cmd = "cargo test"` under the `[criteria.tests-pass]` (or similar) section
10. Run `assay gate run my-chunk`
11. **Expected:** Gate runs without error about missing cmd

### 2. CLI wizard: criterion without cmd (Enter to skip)

1. `assay plan`
2. Enter goal, 1 chunk, criterion name, then **press Enter immediately** at the cmd prompt
3. Confirm: `cat .assay/specs/<chunk>/gates.toml`
4. **Expected:** No `cmd` key under the criterion — same output as pre-S04 wizard

### 3. CLI wizard: mixed criteria (some with cmd, some without)

1. `assay plan` with 1 chunk, 2 criteria
2. Criterion 1: name "passes", cmd `cargo test`
3. Criterion 2: name "lints", cmd empty (Enter)
4. **Expected:** gates.toml has `cmd = "cargo test"` only for criterion 1; criterion 2 has no cmd key

### 4. TUI wizard: cmd collection

1. Launch `assay-tui`
2. Press `n` to open the wizard
3. Navigate through goal and chunk count steps
4. Enter a criterion name → **Expected:** prompt changes to "Command (Enter to skip):"
5. Enter a command string
6. **Expected:** Prompt returns to criterion name input; enter another name → skips cmd with Enter
7. Submit the wizard
8. **Expected:** Generated spec's `gates.toml` has cmd for the first criterion, no cmd for the second

### 5. MCP spec_create: structured criteria with cmd

Using an MCP client (or assay-mcp integration test):

```json
{
  "tool": "spec_create",
  "params": {
    "milestone": "my-milestone",
    "name": "my-spec",
    "criteria": [
      { "name": "tests pass", "cmd": "cargo test" },
      "lints clean"
    ]
  }
}
```

**Expected:** Generated `gates.toml` has `cmd = "cargo test"` for first criterion; no cmd field for second criterion (plain string → description only).

## Edge Cases

### Blank command at TUI cmd prompt

1. In TUI wizard criteria step, after entering a criterion name, press Enter immediately at the cmd prompt
2. **Expected:** `criteria_awaiting_cmd` resets to false; wizard proceeds to next criterion name prompt; generated spec has no `cmd` field for that criterion

### MCP: backward compat — plain strings only

Send `"criteria": ["passes", "lints clean"]` (all plain strings) to `spec_create`.

**Expected:** Gates.toml produced without any `cmd` fields — identical to pre-S04 behavior

## Failure Signals

- `gates.toml` missing `cmd` field after entering a command → wire-through regression in `write_gates_toml`
- `gates.toml` has `cmd` field when command was skipped → `None` not filtering correctly in serde skip
- `gate run` failing with "missing cmd" error on a spec produced by the wizard → cmd not written to TOML
- TUI wizard not showing "Command (Enter to skip):" prompt → `criteria_awaiting_cmd` flag not set
- MCP `spec_create` rejecting plain string criteria → `CriterionOrString` untagged deserialization broken

## Requirements Proved By This UAT

- R082 — `assay plan` wizard collects optional cmd per criterion; generated `gates.toml` has `cmd` when provided; `gate run` succeeds on wizard output without manual editing; CLI, TUI, and MCP surfaces all accept cmd input; backward compatibility confirmed for plain-string MCP criteria

## Not Proven By This UAT

- Automated contract tests prove the cmd field round-trip (wizard input → gates.toml) — the UAT verifies the interactive UX only
- Real `gate run` execution against a real CI/CD command (e.g., `cargo test` actually running and passing) is outside scope — S04 proves the spec is runnable, not that the underlying command succeeds
- MCP schema generation correctness for `CriterionOrString` — verified by unit tests, not UAT

## Notes for Tester

- The cmd prompt appears **after** each criterion name, before the next name is requested. If you see two consecutive "Criterion name:" prompts, the cmd sub-step is not engaged.
- Empty Enter at the cmd prompt is intentional skip — do not enter a space or placeholder.
- The TUI wizard's "Command (Enter to skip):" prompt is rendered in the same input area as the criterion name — the prompt text is the signal that you're in cmd sub-step.
- `assay gate run` requires the spec's `cmd` field to be a valid shell command that exits 0 to pass. Testing with `cargo test` requires a Rust project in scope.
