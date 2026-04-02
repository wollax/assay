---
estimated_steps: 7
estimated_files: 2
---

# T01: Add KubernetesConfig struct, extend validate(), and write manifest tests

**Slice:** S01 — Manifest Extension
**Milestone:** M005

## Description

Extend `manifest.rs` with the `KubernetesConfig` struct and `JobManifest.kubernetes` optional field. Extend `validate()` to accept `"kubernetes"` as a valid runtime and enforce the bidirectional cross-guard: if `runtime == "kubernetes"` then a `[kubernetes]` block with non-empty `namespace` and `ssh_key_env` is required; if `runtime != "kubernetes"` and a `[kubernetes]` block is present, that is also an error. Fix the existing `test_validate_runtime_unknown_rejected` test (which currently uses `"kubernetes"` as the unknown value — it must be changed to `"podman"` so it still tests the right thing after `"kubernetes"` is added to the allowlist). Add `kube` and `k8s-openapi` to `Cargo.toml` to unblock T02.

## Steps

1. Add to `[dependencies]` in `crates/smelt-core/Cargo.toml`:
   ```toml
   kube = { version = "3", default-features = true }
   k8s-openapi = { version = "0.27", features = ["v1_32"] }
   ```
   Do NOT add `ws` feature on `kube` yet — that is for S02.

2. In `manifest.rs`, add `KubernetesConfig` struct directly after the `ForgeConfig` import block:
   - `#[derive(Debug, Deserialize)]`
   - `#[serde(deny_unknown_fields)]`
   - Doc comment on the struct: `/// Configuration for the Kubernetes runtime provider.`
   - 7 fields with doc comments and `#[serde(default)]` on all `Option` fields:
     - `pub namespace: String` — `/// Kubernetes namespace for the Pod and SSH Secret.`
     - `pub context: Option<String>` — `/// kubeconfig context to use; uses ambient context if absent.`
     - `pub ssh_key_env: String` — `/// Name of the env var containing the SSH private key.`
     - `pub cpu_request: Option<String>` — `/// CPU request for the agent container (e.g. "500m").`
     - `pub memory_request: Option<String>` — `/// Memory request for the agent container (e.g. "512Mi").`
     - `pub cpu_limit: Option<String>` — `/// CPU limit for the agent container.`
     - `pub memory_limit: Option<String>` — `/// Memory limit for the agent container.`

3. Add `pub kubernetes: Option<KubernetesConfig>` to `JobManifest` struct, positioned after `pub forge: Option<ForgeConfig>` and before `pub services: Vec<ComposeService>`:
   ```rust
   /// Optional Kubernetes runtime configuration.
   #[serde(default)]
   pub kubernetes: Option<KubernetesConfig>,
   ```

4. In `validate()`, extend `VALID_RUNTIMES` from `&["docker", "compose"]` to `&["docker", "compose", "kubernetes"]`.

5. In `validate()`, add cross-field guard after the existing `services` runtime guard:
   ```rust
   // kubernetes block requires kubernetes runtime and vice versa
   if self.environment.runtime == "kubernetes" {
       match &self.kubernetes {
           None => errors.push("kubernetes: `runtime = \"kubernetes\"` requires a `[kubernetes]` block".to_string()),
           Some(k) => {
               if k.namespace.trim().is_empty() {
                   errors.push("kubernetes.namespace: must not be empty".to_string());
               }
               if k.ssh_key_env.trim().is_empty() {
                   errors.push("kubernetes.ssh_key_env: must not be empty".to_string());
               }
           }
       }
   } else if self.kubernetes.is_some() {
       errors.push(format!(
           "kubernetes: `[kubernetes]` block requires `runtime = \"kubernetes\"`, got `{}`",
           self.environment.runtime
       ));
   }
   ```

6. Fix `test_validate_runtime_unknown_rejected`: change `runtime = "kubernetes"` to `runtime = "podman"` in the TOML string. Update the assertion comment if needed.

7. Add the following tests to the `#[cfg(test)]` block at the bottom of `manifest.rs`:
   - `test_kubernetes_roundtrip_present`: valid kubernetes TOML with all 7 fields set; assert `manifest.kubernetes.is_some()` and check field values.
   - `test_kubernetes_roundtrip_absent`: standard docker-runtime manifest without `[kubernetes]`; assert `manifest.kubernetes.is_none()`.
   - `test_validate_kubernetes_runtime_requires_block`: `runtime = "kubernetes"` but no `[kubernetes]` block; `validate()` returns error containing `"kubernetes"`.
   - `test_validate_kubernetes_block_requires_runtime`: `runtime = "docker"` + `[kubernetes]` block; error contains `"kubernetes"`.
   - `test_validate_kubernetes_empty_namespace`: `runtime = "kubernetes"` + `[kubernetes]` with `namespace = ""`; error contains `"namespace"`.
   - `test_validate_kubernetes_empty_ssh_key_env`: `runtime = "kubernetes"` + `[kubernetes]` with `ssh_key_env = ""`; error contains `"ssh_key_env"`.
   - `test_validate_kubernetes_valid`: fully valid kubernetes manifest; `validate()` returns `Ok(())`.

## Must-Haves

- [ ] `KubernetesConfig` struct compiles with `deny_unknown_fields` and all 7 fields documented
- [ ] `JobManifest.kubernetes: Option<KubernetesConfig>` — existing manifests without `[kubernetes]` parse identically (no breaking change)
- [ ] `validate()` accepts `"kubernetes"` as a valid runtime value
- [ ] Cross-guard (both directions) implemented and tested: missing block on kubernetes runtime → error; block present on non-kubernetes runtime → error
- [ ] Empty `namespace` and empty `ssh_key_env` each produce a separate validation error
- [ ] `test_validate_runtime_unknown_rejected` updated to use `"podman"` and still passes
- [ ] All 7 new tests pass
- [ ] `cargo test -p smelt-core` exits 0 with no FAILED lines

## Verification

- `cargo test -p smelt-core 2>&1 | tail -10` — test result: ok with the 7 new tests present
- `cargo test -p smelt-core -- test_validate_runtime_unknown_rejected 2>&1` — passes (rejects "podman", not "kubernetes")
- `cargo test -p smelt-core -- kubernetes 2>&1` — all 7 new tests pass
- `cargo test --workspace 2>&1 | grep FAILED` — empty output (no regressions)

## Observability Impact

- Signals added/changed: `validate()` error messages for kubernetes runtime guards — structured, human-readable strings; downstream code can pattern-match on error text for testing
- How a future agent inspects this: `cargo test -p smelt-core -- kubernetes --nocapture` shows all kubernetes test output; validation errors surface in `smelt run` output when manifest is invalid
- Failure state exposed: Validation errors include field names (`kubernetes.namespace`, `kubernetes.ssh_key_env`) making it unambiguous which field is wrong

## Inputs

- `crates/smelt-core/src/manifest.rs` — existing `ForgeConfig` pattern (line ~30), `validate()` (line ~192), `VALID_RUNTIMES`, `test_validate_runtime_unknown_rejected`
- `crates/smelt-core/Cargo.toml` — to add deps

## Expected Output

- `crates/smelt-core/Cargo.toml` — `kube` and `k8s-openapi` added to `[dependencies]`
- `crates/smelt-core/src/manifest.rs` — `KubernetesConfig` struct, `JobManifest.kubernetes` field, extended `validate()`, 7 new tests, fixed `test_validate_runtime_unknown_rejected`
