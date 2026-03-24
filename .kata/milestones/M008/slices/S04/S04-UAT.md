# S04: Gate History Analytics Engine and CLI — UAT

**Milestone:** M008
**Written:** 2026-03-24

## UAT Type

- UAT mode: live-runtime
- Why this mode is sufficient: Analytics aggregates real gate history — meaningful results require actual gate runs against a project with specs and history records

## Preconditions

- A project directory with `.assay/` initialized (`assay init`)
- At least one spec with gate criteria defined
- At least one gate run completed (`assay gate run <spec>`) so `.assay/results/` has history
- At least one milestone with completed chunks (for velocity data)

## Smoke Test

Run `assay history analytics` from the project root. Should see two tables: "Gate Failure Frequency" and "Milestone Velocity" with data from your gate runs.

## Test Cases

### 1. Structured text output

1. `cd` to a project with gate history
2. Run `assay history analytics`
3. **Expected:** Two formatted tables — "Gate Failure Frequency" (columns: Spec, Criterion, Fails, Runs, Rate, Enforcement) and "Milestone Velocity" (columns: Milestone, Chunks, Days, Rate). Failure rates colored red (>50%), yellow (>0%), green (0%).

### 2. JSON output

1. Run `assay history analytics --json`
2. Pipe through `jq .` or similar JSON validator
3. **Expected:** Valid JSON with `failure_frequency` array, `milestone_velocity` array, and `unreadable_records` integer field.

### 3. Non-project directory

1. `cd /tmp`
2. Run `assay history analytics`
3. **Expected:** Error message "not an Assay project" with exit code 1.

### 4. Empty project

1. Run `assay init` in a fresh directory
2. Run `assay history analytics`
3. **Expected:** Empty tables (no crash, no error). Both sections present but with no data rows.

## Edge Cases

### Corrupt history records

1. Manually corrupt a JSON file in `.assay/results/<spec>/`
2. Run `assay history analytics`
3. **Expected:** Analytics still produces results for valid records. Unreadable record count shown in footer. Stderr warning about the corrupt file.

### Many gate runs

1. Run gates multiple times across multiple specs
2. Run `assay history analytics`
3. **Expected:** Failure frequency aggregates correctly across runs. Same criterion from same spec shows cumulative counts.

## Failure Signals

- Crash or panic on any input (including empty/corrupt data)
- Missing "Gate Failure Frequency" or "Milestone Velocity" section headings
- JSON output that fails to parse as valid JSON
- Silent data loss (records skipped without `unreadable_records` count)

## Requirements Proved By This UAT

- R059 (Gate history analytics) — CLI portion: `assay history analytics` outputs failure frequency and milestone velocity from real gate history data; `--json` provides machine-readable output

## Not Proven By This UAT

- R059 TUI portion — TUI analytics screen (S05) not yet built
- Performance at scale — no testing with thousands of history records
- Concurrent access — analytics reads are not locked against concurrent gate writes

## Notes for Tester

- The analytics output is most interesting with a mix of passing and failing gate runs across multiple specs
- If no milestones have completed chunks, the velocity table will be empty (this is correct behavior)
- `NO_COLOR=1` disables ANSI coloring in text output
