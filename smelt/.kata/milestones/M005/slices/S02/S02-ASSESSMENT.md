# S02 Post-Slice Roadmap Assessment

**Assessed after:** S02 — KubernetesProvider Lifecycle
**Date:** 2026-03-23
**Verdict:** Roadmap unchanged — S03 and S04 proceed as planned

## Risk Retirement Status

| Risk | Planned Retirement | Actual Status |
|------|-------------------|---------------|
| `kube` exec WebSocket | S02 | ✅ Retired — `exec()` and `exec_streaming()` via `AttachedProcess` implemented and build-verified |
| Pod readiness detection | S02 | ✅ Retired — 60×2s polling loop with init-done + main-running + image-pull-backoff fast-fail implemented |
| Push-from-Pod result collection | S03 | ⏳ Not yet — `collect()` is a no-op; S03 owns this |
| SSH file permissions | S02 (live test) | ⚠️ Code correct (mode `0400` set in S01 pod spec; Secret created by S02 provision()); `test_k8s_ssh_permissions` test written but not live-run (no kind cluster). Will be implicitly retired by S03's end-to-end integration test — git clone over SSH only succeeds with correct file permissions |

## Boundary Map Accuracy

S02 delivered exactly what the boundary map specified. Key check:

- `SMELT_GIT_REMOTE` injection is S03's work (correctly deferred) — S02 forward intelligence documents the injection point (`provision()` or `generate_pod_spec()`)
- `collect()` is a no-op as planned — S03 owns the Phase 8 `git fetch` in `run.rs`
- `ContainerId` format `"<namespace>/<pod-name>"` is stable — both `exec()` and `teardown()` use `parse_container_id()`; S03 and S04 must not assume a different format
- `generate_pod_spec()` signature (`ssh_private_key: &str` unused) is unchanged from S01 — S03 does not need to change it; SSH key bytes are in the Secret that `provision()` creates

## S03 Readiness Check

S03 can proceed immediately. It consumes:
- ✅ `KubernetesProvider` full impl (provision, exec, teardown) — complete
- ✅ `PodState` — complete, tracks `namespace/pod_name/secret_name`
- ✅ SSH credential injection at provision time (Secret created with key from env var) — complete

S03's two implementation points:
1. Inject `SMELT_GIT_REMOTE` env var into the agent container env at provision time (either via a new param to `provision()` or directly in `generate_pod_spec()`)
2. Add `run.rs` Phase 8 branch: `if manifest.environment.runtime == "kubernetes" { git fetch origin <target_branch> }`

## Requirement Coverage

R021 (Multi-machine coordination via Kubernetes): status remains `active/mapped`. S02 advanced it significantly (KubernetesProvider is a complete RuntimeProvider), but full validation requires S03 (push-from-Pod end-to-end) and S04 (CLI dispatch). Coverage is sound — S03 and S04 are both scoped to R021.

All 15 previously validated requirements (R001–R015, R020) are unaffected — S02 touched only K8s code paths with zero regressions in the workspace test suite (154 unit tests, 0 failures).

## Decisions Added in S02

- **D093** — Sequential stdout→stderr in exec methods (FnMut cannot be shared across concurrent branches)
- **D094** — `teardown()` derives `secret_name` from ContainerId formula, not PodState lookup

Both are stable and do not require roadmap changes.

## Conclusion

Remaining slices S03 and S04 are correctly scoped, correctly ordered, and their boundary contracts accurately reflect what S02 actually built. No slice changes, reorderings, or splits are warranted.
