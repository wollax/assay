# S01 Roadmap Assessment

**Assessed after:** S01 (Manifest Extension) completion
**Decision:** Roadmap unchanged — no adjustments needed

## Risk Retirement

S01 was `risk:low` with no assigned risks to retire. All four milestone risks remain correctly assigned to remaining slices:

- `kube` crate exec WebSocket → S02 (unchanged)
- Push-from-Pod result collection → S03 (unchanged)
- Pod readiness detection → S02 (unchanged)
- SSH file permissions → S02 (unchanged)

No new risks surfaced during S01 execution.

## Boundary Contract Accuracy

Every output in the S01 → S02/S03/S04 boundary map was delivered exactly as specified:

- `KubernetesConfig` struct (7 fields, deny_unknown_fields) ✓
- `JobManifest.kubernetes: Option<KubernetesConfig>` ✓
- Bidirectional `validate()` cross-guard ✓
- `generate_pod_spec()` pure function → `k8s_openapi::api::core::v1::Pod` ✓
- `KubernetesProvider` stub wired into `lib.rs` ✓
- `kube = "3"` + `k8s-openapi = "0.27"` deps ✓
- 10 kubernetes unit tests ✓

S02 forward intelligence confirmed: Secret name is `smelt-ssh-<job-name>` (must match between Secret creation and Pod volume reference); `ws` feature intentionally absent from S01 — S02 must add `kube = { version = "3", features = ["ws"] }`.

## Success Criterion Coverage

| Criterion | Remaining Owner(s) |
|-----------|-------------------|
| `smelt run` creates Pod, runs Assay, tears down | S02, S03, S04 |
| Result branch available on remote after Pod exits | S03 |
| `--dry-run` exits 0, shows `── Kubernetes ──` section | S04 |
| SSH credentials never visible in container env vars | S02 |
| `runtime = "docker"` / `runtime = "compose"` unchanged | S04 |
| Integration tests against real kind cluster pass | S02, S03 |

All six criteria have at least one remaining owning slice. Coverage check passes.

## Requirement Coverage

R021 (Multi-machine coordination via Kubernetes) is the sole active requirement. S01 advanced its foundation — manifest schema and Pod spec generator are now locked contracts for downstream slices. R021 validation remains correctly assigned to S02 (primary) with S03 and S04 as supporting. No new requirements surfaced; none were invalidated or re-scoped.

## Conclusion

The remaining slices (S02, S03, S04) should proceed as planned. Slice ordering, risk assignment, boundary contracts, and requirement coverage are all sound. No roadmap edits needed.
