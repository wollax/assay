# S02: KubernetesProvider Lifecycle — UAT

**Milestone:** M005
**Written:** 2026-03-23

## UAT Type

- UAT mode: live-runtime
- Why this mode is sufficient: This slice implements K8s cluster operations — Secret/Pod creation, WebSocket exec, teardown. The automated integration tests (`SMELT_K8S_TEST=1`) are the primary proof mechanism; human UAT confirms they pass in a real cluster environment and that the namespace is clean after the run.

## Preconditions

- `kind` or `minikube` cluster running and reachable via `kubectl`
- `smelt` namespace exists: `kubectl create namespace smelt` (or already created)
- SSH key available: `export SMELT_TEST_SSH_KEY="$(cat ~/.ssh/id_rsa)"` (or any valid SSH private key)
- `SMELT_K8S_TEST=1` environment variable set
- `cargo` available and workspace compiles cleanly

## Smoke Test

Run the workspace tests without a cluster — all 4 k8s tests should appear as `ignored` (not failures):

```
cargo test --workspace
```

Expected: `4 ignored; 0 failed` in the k8s_lifecycle test output.

## Test Cases

### 1. Full Lifecycle: Provision → Exec → Teardown

```
export SMELT_TEST_SSH_KEY="$(cat ~/.ssh/id_rsa)"
export SMELT_K8S_TEST=1
cargo test -p smelt-cli --test k8s_lifecycle -- --include-ignored test_k8s_provision_exec_teardown 2>&1
```

1. Test provisions a Pod with SSH Secret in the `smelt` namespace
2. Execs `echo hello` in the running agent container
3. Asserts exit code 0 and stdout contains "hello"
4. Tears down Pod and Secret
5. **Expected:** `test test_k8s_provision_exec_teardown ... ok`

### 2. Streaming Exec Callback

```
SMELT_TEST_SSH_KEY="$(cat ~/.ssh/id_rsa)" SMELT_K8S_TEST=1 \
  cargo test -p smelt-cli --test k8s_lifecycle -- --include-ignored test_k8s_exec_streaming_callback 2>&1
```

1. Provisions a Pod
2. Calls `exec_streaming` with a callback that accumulates chunks
3. **Expected:** `test test_k8s_exec_streaming_callback ... ok` — callback fired, content contains "streaming-hello"

### 3. SSH File Permissions

```
SMELT_TEST_SSH_KEY="$(cat ~/.ssh/id_rsa)" SMELT_K8S_TEST=1 \
  cargo test -p smelt-cli --test k8s_lifecycle -- --include-ignored test_k8s_ssh_file_permissions 2>&1
```

1. Provisions a Pod (SSH key mounted at `/root/.ssh/id_rsa`)
2. Execs `stat /root/.ssh/id_rsa` inside the container
3. **Expected:** `test test_k8s_ssh_file_permissions ... ok` — stat output contains `0400`

### 4. Readiness Confirmation

```
SMELT_TEST_SSH_KEY="$(cat ~/.ssh/id_rsa)" SMELT_K8S_TEST=1 \
  cargo test -p smelt-cli --test k8s_lifecycle -- --include-ignored test_k8s_readiness_confirmed 2>&1
```

1. Provisions a Pod
2. Immediately execs `echo ready` — if provision() returns before the main container is Running, this exec will fail
3. **Expected:** `test test_k8s_readiness_confirmed ... ok` — exit code 0

### 5. Full Suite

```
SMELT_TEST_SSH_KEY="$(cat ~/.ssh/id_rsa)" SMELT_K8S_TEST=1 \
  cargo test -p smelt-cli --test k8s_lifecycle -- --include-ignored 2>&1
```

**Expected:** `4 passed; 0 failed; 0 ignored`

## Edge Cases

### Namespace Cleanup After Tests

After running all 4 tests:

```
kubectl get pods -n smelt
kubectl get secrets -n smelt
```

**Expected:** No `smelt-*` pods or `smelt-ssh-*` secrets in the namespace. If any remain, teardown failed.

### Graceful Skip Without Cluster

Without `SMELT_K8S_TEST` set:

```
cargo test -p smelt-cli --test k8s_lifecycle -- --include-ignored
```

**Expected:** `4 passed` (all tests skip via `k8s_provider_or_skip()` returning `None`); no panic, no connection errors.

### Workspace Regression Check

```
cargo test --workspace
```

**Expected:** All existing tests pass; k8s tests appear as `4 ignored`; 0 failures.

## Failure Signals

- Tests fail with timeout errors → `kubectl describe pod smelt-<name> -n smelt` to see readiness status; check init container exit code
- `SMELT_TEST_SSH_KEY not set` error → export the variable before running tests
- `failed to build kube client` → check `kubectl cluster-info` to confirm cluster is reachable
- Tests fail with `ImagePullBackOff` → check `environment.image` in `k8s_manifest()` is pullable from the cluster
- SSH permission test fails → `kubectl exec smelt-smelt-test -n smelt -- stat /root/.ssh/id_rsa` to inspect mode directly
- Pod not deleted after teardown → `kubectl delete pod smelt-smelt-test -n smelt --force` and check `teardown()` error logs with `RUST_LOG=smelt_core=warn`

## Requirements Proved By This UAT

- R021 (partially) — `KubernetesProvider` lifecycle operations proven against a real cluster: provision creates resources, exec runs commands via WebSocket, teardown cleans up; `kube` exec WebSocket risk and Pod readiness detection risk retired

## Not Proven By This UAT

- R021 full validation — requires S03 (push-from-Pod result collection) and S04 (CLI dispatch via `smelt run`) to be complete before R021 can move to `validated`
- End-to-end `smelt run examples/job-manifest-k8s.toml` — deferred to S04-UAT.md
- SSH git clone inside init container (actual repo clone with real SSH key) — init container uses `sleep 3600` in test manifest; real clone proof is S03's work
- PR creation from K8s path — deferred to S03/S04

## Notes for Tester

- Tests use `job.name = "smelt-test"` so pods are named `smelt-smelt-test` — this is not a typo; it's `smelt-<job-name>`
- `pre_clean_k8s()` deletes any leftover `smelt-smelt-test` pod and `smelt-ssh-smelt-test` secret before each test — orphans from a prior failed run won't cause name-collision errors
- The test image in `k8s_manifest()` uses `environment.image` from a test manifest — confirm the image is pullable from your cluster or update `k8s_manifest()` in the test file to use a locally-available image
- `RUST_LOG=smelt_core=info` during test runs shows the provision lifecycle trace (Secret created, Pod created, pod ready, teardown complete)
