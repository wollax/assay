# S03 Post-Slice Roadmap Assessment

**Assessed:** 2026-03-22
**Slice completed:** S03 — ComposeProvider Lifecycle
**Remaining slices:** S04 (CLI Integration + Dry-Run)

## Verdict: Roadmap is unchanged

S04 scope and risk rating remain accurate. No rewrite needed.

## Risk Retirement

All three key risks from the roadmap are now retired:

| Risk | Retirement target | Status |
|------|-------------------|--------|
| TOML → YAML type fidelity | S02 snapshot tests | ✅ Retired |
| `docker compose ps` stability | S03 integration tests | ✅ Retired |
| ComposeProvider internal state | S03 design + integration tests | ✅ Retired |

## Success-Criterion Coverage

| Criterion | Remaining owner |
|-----------|----------------|
| `smelt run` provisions stack, waits healthy, runs Assay, tears down; `docker ps` empty after | S04 |
| Service containers reachable by name from agent (proven in S03; end-to-end `smelt run` proof) | S04 |
| `--dry-run` shows `── Compose Services ──`, exits 0 without Docker | S04 |
| Ctrl+C tears down full Compose stack cleanly | S04 |
| Any Compose service field passes through unchanged (proven S01/S02; S04 closes end-to-end) | S04 |
| `runtime = "docker"` path completely unchanged | S04 |

All six success criteria have S04 as their remaining owner. Coverage is sound.

## Boundary Map Accuracy

S03 introduced two correctness fixes that changed the YAML shape produced by `generate_compose_file()`:

- **D082:** Custom `networks:` section removed from smelt-agent and top-level; relying on Docker Compose default project network for DNS resolution.
- **D083:** `command: [sleep, "3600"]` added to smelt-agent service to prevent immediate exit.

Both fixes are captured in 6 updated snapshot tests in `smelt-core/src/compose.rs`, which are now authoritative for the correct YAML shape. S04's boundary map entry consumes `ComposeProvider: RuntimeProvider` (not the YAML internals), so no boundary map updates are required.

## Requirement Coverage

R020 (Docker Compose runtime for multi-service environments) is now validated by S03 integration tests. S04 completes the end-to-end `smelt run` proof but does not change R020's validation status — it was already marked validated after S03.

All 15 validated requirements remain validated. No requirement ownership changes.

## Forward Notes for S04

- `ComposeProvider::new()` is failable — S04's `run.rs` dispatch must handle the constructor error.
- Compose project name is `smelt-{job_name}`; Docker Compose auto-creates the default network as `smelt-{job_name}_default`. The dry-run output should not reference a custom network name.
- The `smelt-agent` keep-alive pattern (`sleep 3600`) is now mandatory in the generated YAML — S04 example manifest should use an image that accepts this command (alpine:3 does).
