---
id: T02
parent: S01
milestone: M005
provides:
  - generate_pod_spec() pure function returning k8s_openapi::api::core::v1::Pod with init container, emptyDir, SSH Secret volume (defaultMode 256), and resource limits/requests
  - KubernetesProvider stub struct with full RuntimeProvider impl (all 5 methods todo!())
  - pub mod k8s + pub use k8s::KubernetesProvider wired into lib.rs
  - examples/job-manifest-k8s.toml — validated kubernetes example manifest
  - 3 snapshot tests: pod spec shape, misuse guard (Err on missing kubernetes block), resource limits
key_files:
  - crates/smelt-core/src/k8s.rs
  - crates/smelt-core/src/lib.rs
  - examples/job-manifest-k8s.toml
key_decisions:
  - ssh_private_key parameter suppressed with let _ = ssh_private_key; — param reserved for S02 Secret creation, not used by generate_pod_spec itself
  - defaultMode: Some(256) chosen for SSH key volume (0o400 = user-read-only; SSH client requires this)
  - KubernetesProvider uses async fn syntax directly in impl (not the future-returning form) following Rust 2024 RPITIT stabilization
  - ResourceRequirements.limits and .requests fields set to None when BTreeMap is empty (clean serialization)
patterns_established:
  - generate_pod_spec follows generate_compose_file signature reference pattern from compose.rs (pure function, manifest + named params → Result<T>)
  - Snapshot tests serialize to serde_json::to_string_pretty and assert substring presence for field-level verification
observability_surfaces:
  - cargo test -p smelt-core -- generate_pod_spec --nocapture — prints full JSON snapshot; failing tests show which JSON substring is absent
  - SmeltError::provider("k8s", "generate_pod_spec called without [kubernetes] config block") — unambiguous error on misuse
  - cargo run --bin smelt -- run examples/job-manifest-k8s.toml --dry-run — confirms CLI parsing path end-to-end
duration: 25min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
blocker_discovered: false
---

# T02: Implement generate_pod_spec(), KubernetesProvider stub, lib.rs wiring, and example manifest

**generate_pod_spec() returns a fully-typed k8s_openapi Pod with init container (alpine/git), emptyDir workspace volume, SSH Secret volume (mode 0400), and optional resource limits; KubernetesProvider stub wired into lib.rs; all 3 snapshot tests passing; --dry-run exits 0**

## What Happened

Created `crates/smelt-core/src/k8s.rs` with:

1. **`generate_pod_spec(manifest, job_name, ssh_private_key) -> Result<Pod>`**: Pure synchronous function that builds a `k8s_openapi::api::core::v1::Pod` value. The function has a misuse guard that returns `SmeltError::provider("k8s", ...)` when `manifest.kubernetes` is `None`. The SSH Secret volume uses `default_mode: Some(256)` (0o400) to satisfy the SSH client's key permission requirement. An `alpine/git` init container clones the repo via SSH into an emptyDir `/workspace` volume. The main container uses `manifest.environment.image` with the workspace mounted. Resource requests and limits are built from `KubernetesConfig` optional fields using `BTreeMap<String, Quantity>`, only including keys where `Option` is `Some`. The `ssh_private_key` parameter is suppressed with `let _ = ssh_private_key;` as it is reserved for S02 Secret creation.

2. **`KubernetesProvider` stub**: Zero-field struct with full `RuntimeProvider` impl where all 5 methods call `todo!("KubernetesProvider not implemented until S02")`. All public items are documented.

3. **3 snapshot tests** in `#[cfg(test)]` mod: `test_generate_pod_spec_snapshot` (checks initContainers, alpine/git, defaultMode 256, emptyDir, smelt-ssh-my-job, Never); `test_generate_pod_spec_requires_kubernetes_config` (Err returned when no kubernetes block); `test_generate_pod_spec_resource_limits` (requests and limits present when cpu_request/memory_limit set).

Wired `pub mod k8s` and `pub use k8s::KubernetesProvider` into `lib.rs` following the `ComposeProvider` pattern.

Created `examples/job-manifest-k8s.toml` with all required sections (job, environment, credentials, session, merge, kubernetes).

## Verification

- `cargo test -p smelt-core -- generate_pod_spec 2>&1` → 3 tests pass (snapshot, misuse guard, resource limits)
- `cargo test --workspace 2>&1 | grep -E "^(test result|FAILED)"` → all green, zero FAILED lines (9 test result lines, all ok)
- `cargo run --bin smelt -- run examples/job-manifest-k8s.toml --dry-run 2>&1` → exits 0, prints execution plan with runtime: kubernetes
- `cargo doc --package smelt-core --no-deps 2>&1 | grep missing_docs` → empty (no missing doc warnings)

## Diagnostics

- `cargo test -p smelt-core -- generate_pod_spec --nocapture` prints full JSON snapshot; failing tests show which JSON substring is absent
- `SmeltError::provider("k8s", "generate_pod_spec called without [kubernetes] config block")` is the error returned on misuse — contains "kubernetes" for test assertion
- `cargo run --bin smelt -- run examples/job-manifest-k8s.toml --dry-run` confirms the full CLI parsing path including validation

## Deviations

None — implementation followed the task plan exactly. The `async fn` syntax used in `impl RuntimeProvider for KubernetesProvider` (Rust 2024 RPITIT) matches the pattern used elsewhere in the codebase (compose.rs, docker.rs).

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/k8s.rs` — new module: generate_pod_spec(), KubernetesProvider stub, 3 snapshot tests
- `crates/smelt-core/src/lib.rs` — added pub mod k8s and pub use k8s::KubernetesProvider
- `examples/job-manifest-k8s.toml` — new kubernetes example manifest
