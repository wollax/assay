# S04: CLI Integration + Dry-Run — UAT

**Milestone:** M005
**Written:** 2026-03-23

## UAT Type

- UAT mode: mixed (artifact-driven for dry-run; live-runtime for real K8s execution)
- Why this mode is sufficient: Dry-run correctness is fully provable by automated tests (27/27 pass). Live-runtime execution against a real kind cluster + real Assay image requires human infrastructure that was not available during slice execution. The automated portion proves the dispatch wiring and dry-run output; the human portion proves end-to-end K8s execution.

## Preconditions

**For dry-run tests (automated — already passing):**
- `cargo test --workspace` passes (confirmed; all 27 dry-run tests green).
- `examples/job-manifest-k8s.toml` exists (confirmed).

**For live K8s tests:**
- A kind cluster is running locally (`kind create cluster` if needed; `kubectl cluster-info` should respond).
- `kubectl` is configured to reach the cluster (`kubectl get nodes` returns Ready).
- An SSH key with push access to a test GitHub repo is available (env var pointed to by `ssh_key_env` in the manifest).
- A real Assay binary is reachable as `assay` on PATH (or inside the agent container image).
- `ANTHROPIC_API_KEY` (or equivalent) is set for the agent container.
- The agent container image (`ghcr.io/example/assay-agent:latest` or your actual image) is pullable from the cluster.

## Smoke Test

```sh
cargo run --bin smelt -- run examples/job-manifest-k8s.toml --dry-run
```

Expected: exits 0; stdout contains `── Kubernetes ──`, `Namespace:   smelt`, `Context:     ambient`, and all 4 resource fields.

## Test Cases

### 1. Dry-run shows kubernetes section

1. `cargo run --bin smelt -- run examples/job-manifest-k8s.toml --dry-run`
2. **Expected:** Exit 0; stdout contains:
   ```
   ── Kubernetes ──
     Namespace:   smelt
     Context:     ambient
     CPU req:     500m
     Mem req:     512Mi
     CPU limit:   2000m
     Mem limit:   2Gi
   ```

### 2. Docker runtime is unchanged

1. `cargo run --bin smelt -- run examples/job-manifest.toml --dry-run`
2. **Expected:** Exit 0; stdout does NOT contain `── Kubernetes ──`; shows `── Environment ──` with `Runtime: docker` as before.

### 3. Compose runtime is unchanged

1. `cargo run --bin smelt -- run examples/job-manifest-compose.toml --dry-run`
2. **Expected:** Exit 0; stdout does NOT contain `── Kubernetes ──`; shows `── Compose Services ──` as before.

### 4. Live K8s run (requires kind cluster)

1. Update `examples/job-manifest-k8s.toml` with a real GitHub repo and SSH key env var.
2. Ensure the SSH key env var is set in the shell.
3. `cargo run --bin smelt -- run examples/job-manifest-k8s.toml`
4. Monitor with `kubectl get pods -n smelt` in another terminal.
5. **Expected:**
   - A Pod named `smelt-<job-name>-*` appears in the `smelt` namespace.
   - Init container (`git-clone`) runs and completes; main container (`agent`) starts.
   - Assay runs inside the Pod; gate output streams to stderr.
   - Pod is deleted after completion; `kubectl get pods -n smelt` returns empty.
   - Result branch is pushed to the remote by the agent Pod.
   - `run.rs` Phase 8 fetches the result branch on the host.
   - PR is created (or `--no-pr` skips PR if forge not configured).

### 5. Live K8s run — cluster unreachable

1. Point `KUBECONFIG` at a non-existent cluster (or disconnect from cluster).
2. `cargo run --bin smelt -- run examples/job-manifest-k8s.toml`
3. **Expected:** Exit non-zero; stderr contains `failed to connect to Kubernetes cluster: <cause>` via anyhow chain.

## Edge Cases

### No context set in manifest

1. Verify `examples/job-manifest-k8s.toml` has no `context` field in `[kubernetes]`.
2. `cargo run --bin smelt -- run examples/job-manifest-k8s.toml --dry-run`
3. **Expected:** Dry-run shows `Context: ambient` (fallback label, no cluster touched).

### Context explicitly set in manifest

1. Add `context = "kind-kind"` to `[kubernetes]` in a test manifest copy.
2. `cargo run --bin smelt -- run <test-manifest> --dry-run`
3. **Expected:** Dry-run shows `Context: kind-kind`.

### Unknown runtime rejected

1. Set `runtime = "podman"` in a test manifest.
2. `cargo run --bin smelt -- run <test-manifest>`
3. **Expected:** Exit non-zero; error message lists `docker, compose, kubernetes` as supported runtimes.

## Failure Signals

- Dry-run stdout missing `── Kubernetes ──` → `print_execution_plan()` guard broken or `manifest.kubernetes` is None when it should be Some.
- Live run: `KubernetesProvider::new()` error without context → anyhow `.with_context()` missing in Phase 3 dispatch arm.
- Live run: Pod remains after completion → `teardown()` not called or returning early on error.
- Docker or Compose dry-run shows `── Kubernetes ──` → guard condition incorrect (should be `manifest.kubernetes.is_some()`).

## Requirements Proved By This UAT

- R021 (Multi-machine coordination via Kubernetes) — dry-run portion proves dispatch wiring and plan output; live K8s portion proves end-to-end Assay execution on a real cluster, result branch push from Pod, and PR creation.
- R014 (smelt run --dry-run validates manifest without Docker/K8s) — kubernetes manifests exit 0 and show execution plan without touching the cluster.

## Not Proven By This UAT

- Parallel multi-session K8s scheduling (R023) — deferred to a later milestone.
- Pod scheduling on multi-node clusters — kind is single-node; node selectors, resource quotas, and RBAC isolation are not tested.
- Image pull from private registry — `examples/job-manifest-k8s.toml` uses a public example image; imagePullSecrets are not part of M005 scope.
- Ctrl+C during K8s provision/exec — teardown-on-signal is implemented in `run.rs` via the existing cancellation pattern but is not explicitly exercised in this UAT.

## Notes for Tester

- The dry-run test cases are fully automated and have already passed (`cargo test --workspace` green). Skip them if you only want to validate live K8s behavior.
- For live K8s testing, a kind cluster is the recommended target: `kind create cluster --name smelt-test`.
- `examples/job-manifest-k8s.toml` uses placeholder values (`ghcr.io/example/assay-agent:latest`, `git@github.com:example/repo.git`). You must substitute real values before a live run.
- The `SSH_PRIVATE_KEY` env var (or whatever `ssh_key_env` points to) must contain an SSH private key with push access to the test repo.
- After the live run, verify cleanup: `kubectl get pods -n smelt` and `kubectl get secrets -n smelt` should both return empty.
