# S01: Manifest Extension — UAT

**Milestone:** M005
**Written:** 2026-03-23

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: S01 is a pure contract slice — no cluster, no network, no runtime. All outputs are verifiable via `cargo test`, `cargo doc`, and CLI `--dry-run`. There is no user-visible runtime behavior to exercise manually until S02 introduces real Pod lifecycle operations. The automated tests are the complete proof.

## Preconditions

- Rust toolchain installed (`cargo`, `rustc`)
- `smelt` binary built (`cargo build --bin smelt`)
- No Kubernetes cluster or Docker required

## Smoke Test

```sh
cargo test -p smelt-core -- kubernetes --nocapture
```

Expected: all 10 kubernetes-related tests pass (7 manifest roundtrip/validation, 3 Pod spec snapshot tests).

## Test Cases

### 1. Manifest roundtrip with [kubernetes] block

```sh
cargo test -p smelt-core -- test_kubernetes_roundtrip_present test_kubernetes_roundtrip_absent
```

1. Run the above command.
2. **Expected:** both tests pass; `test_kubernetes_roundtrip_present` confirms all 7 fields parsed; `test_kubernetes_roundtrip_absent` confirms `manifest.kubernetes.is_none()` for manifests without the block.

### 2. Validation cross-guard — runtime mismatch errors

```sh
cargo test -p smelt-core -- test_validate_kubernetes_runtime_requires_block test_validate_kubernetes_block_requires_runtime test_validate_kubernetes_empty_namespace test_validate_kubernetes_empty_ssh_key_env
```

1. Run the above command.
2. **Expected:** all 4 tests pass, proving: missing block → error; mismatched block → error; empty namespace → field-named error; empty ssh_key_env → field-named error.

### 3. Valid kubernetes manifest passes validation

```sh
cargo test -p smelt-core -- test_validate_kubernetes_valid
```

1. Run the above command.
2. **Expected:** test passes; no validation errors for a fully-specified kubernetes manifest.

### 4. Pod spec snapshot test

```sh
cargo test -p smelt-core -- generate_pod_spec --nocapture
```

1. Run the above command.
2. Inspect snapshot output for:
   - `"initContainers"` key present
   - `"alpine/git"` image in init container
   - `"defaultMode": 256` in SSH Secret volume
   - `"emptyDir"` workspace volume
   - `"smelt-ssh-"` prefix in Secret name
   - `"Never"` restart policy
3. **Expected:** all 3 snapshot tests pass; JSON output contains all required substrings.

### 5. Example manifest dry-run

```sh
cargo run --bin smelt -- run examples/job-manifest-k8s.toml --dry-run
```

1. Run the above command.
2. **Expected:** exits 0; execution plan printed with `Runtime: kubernetes`; no parse or validation errors.

### 6. No regressions in workspace

```sh
cargo test --workspace 2>&1 | grep -E "^(test result|FAILED)"
```

1. Run the above command.
2. **Expected:** all test suites show `ok`; zero `FAILED` lines; existing Docker and Compose tests unaffected.

## Edge Cases

### [kubernetes] block without runtime = "kubernetes"

```sh
cargo test -p smelt-core -- test_validate_kubernetes_block_requires_runtime
```

1. Run the test.
2. **Expected:** validation returns error containing "requires `runtime = \"kubernetes\"`".

### generate_pod_spec called without [kubernetes] config

```sh
cargo test -p smelt-core -- test_generate_pod_spec_requires_kubernetes_config
```

1. Run the test.
2. **Expected:** function returns `Err`; error message contains `"kubernetes"`.

### "podman" runtime still rejected

```sh
cargo test -p smelt-core -- test_validate_runtime_unknown_rejected
```

1. Run the test.
2. **Expected:** test passes; unknown runtimes (e.g. "podman") are still rejected after adding "kubernetes" to VALID_RUNTIMES.

## Failure Signals

- Any `FAILED` line in `cargo test --workspace` output
- `cargo run --bin smelt -- run examples/job-manifest-k8s.toml --dry-run` exits non-zero or prints a parse/validation error
- `cargo test -p smelt-core -- generate_pod_spec --nocapture` output missing `"defaultMode": 256` — SSH key permission bug
- Missing `"initContainers"` in snapshot output — init container omitted from Pod spec
- `cargo doc --package smelt-core --no-deps` shows `missing_docs` warnings on k8s.rs items

## Requirements Proved By This UAT

- R021 (partially) — S01 proves the typed manifest contract for Kubernetes runtime: `KubernetesConfig` schema is correct, bidirectional validation guards enforce runtime/block consistency, and `generate_pod_spec()` produces a structurally valid Pod with the required security properties (SSH key mode 0400). The foundation for full R021 validation is established.

## Not Proven By This UAT

- Real cluster connectivity — no kind/minikube cluster is exercised; Pod creation, exec, and teardown are not tested
- SSH key file permission enforcement at the OS level — `defaultMode: 256` is serialized correctly but not verified against a running container (`stat /root/.ssh/id_rsa` deferred to S02)
- `KubernetesProvider` runtime behavior — all 5 `RuntimeProvider` methods are `todo!()` stubs; full implementation is S02
- Push-from-Pod result collection — deferred to S03
- `── Kubernetes ──` dry-run section — deferred to S04
- `AnyProvider::Kubernetes` CLI dispatch — deferred to S04
- End-to-end `smelt run examples/job-manifest-k8s.toml` against a real cluster — deferred to S04-UAT.md

## Notes for Tester

- This is a pure contract slice — all verification is automated. Human review of the snapshot JSON output (step 4) is the only manual check.
- The `-- Kubernetes --` section is intentionally absent from `--dry-run` output until S04. The current output (`Runtime: kubernetes`) is correct behavior for S01.
- `examples/job-manifest-k8s.toml` uses placeholder values (`git@github.com:example/repo.git`, `ghcr.io/example/assay-agent:latest`) — it is designed to parse and validate, not to run against a real cluster.
