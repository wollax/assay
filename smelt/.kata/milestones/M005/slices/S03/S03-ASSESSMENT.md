# S03 Post-Slice Roadmap Assessment

**Assessed after:** S03 — Push-from-Pod Result Collection
**Date:** 2026-03-23
**Verdict:** Roadmap unchanged — S04 proceeds as planned

## Risk Retirement

S03 retired its assigned risk ("push-from-Pod result collection") completely:
- `SMELT_GIT_REMOTE` env var injected into agent container via `generate_pod_spec()`
- `GitOps::fetch_ref()` + `GitCli` implementation with force-refspec (`+<branch>:<branch>`)
- Phase 8 kubernetes fetch block in `run.rs` before `ResultCollector`
- Integration test `test_k8s_push_from_pod_result_collection` (double-guard: `SMELT_K8S_TEST` + `SMELT_TEST_GIT_REMOTE`) proves the full collection path

## Success Criterion Coverage

| Criterion | Status |
|-----------|--------|
| `smelt run examples/job-manifest-k8s.toml` creates Pod, runs Assay, tears down | → S04 |
| Result branch available after Pod exits; PR creation identical to Docker path | → S04 (dispatch wires Phase 8 fetch + ResultCollector end-to-end) |
| `--dry-run` shows `── Kubernetes ──` section | → S04 |
| SSH credentials mounted at `/root/.ssh/id_rsa` mode 0400, never in env vars | ✅ proven by S01+S02 |
| `runtime = "docker"` / `runtime = "compose"` unchanged; zero regressions | → S04 |
| Integration tests against real kind cluster pass | ✅ proven by S02+S03 |

All six criteria have a remaining owner. Coverage passes.

## S04 Boundary Map Accuracy

The S04 boundary map remains accurate with one forward intelligence note from S03: **Phase 8 kubernetes fetch block is already in `run.rs`** — S04 must not add it again. S04's only remaining job is:
- `AnyProvider::Kubernetes(KubernetesProvider)` variant + RuntimeProvider delegation in `run.rs` (Phase 3 dispatch, not Phase 8)
- `print_execution_plan()` `── Kubernetes ──` section for `--dry-run`
- `examples/job-manifest-k8s.toml` working kind-compatible example
- `cargo test --workspace` all green; no regressions in docker/compose paths

## No Changes to Remaining Slices

No new risks emerged. No boundary contracts changed. No reordering needed. R021 (Multi-machine coordination via Kubernetes) remains mapped to S04 as the final validation slice.
