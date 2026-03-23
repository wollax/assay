# S01: Manifest Extension — Research

**Researched:** 2026-03-23
**Domain:** Rust / Kubernetes / TOML manifest parsing
**Confidence:** HIGH

## Summary

S01 is a pure structural slice — no cluster interaction, no async runtime, no WebSocket. The work is: add a `KubernetesConfig` struct to `manifest.rs`, extend `validate()` to cover the new fields and runtime guard, implement a `generate_pod_spec()` function using `k8s-openapi` types, add a `KubernetesProvider` stub module, wire it into `lib.rs`, and add tests. All risks are data-modeling risks (correct `k8s-openapi` field names, SSH Secret volume `defaultMode`), not async/integration risks.

The existing codebase gives three clear reference patterns: `ForgeConfig` for an optional TOML section with `deny_unknown_fields`, `ComposeProjectState` for the provider-internal state map pattern, and `generate_compose_file()` for a pure function returning a typed K8s/serialization value. `generate_pod_spec()` follows the same signature contract as `generate_compose_file()` — pure function, no async, takes `&JobManifest` + extra args, returns `crate::Result<T>`.

The two crates needed are `kube` (v0.98+ spec from M005-ROADMAP, but latest crates.io release is 3.1.0 as of 2026-03) and `k8s-openapi` (v0.27.1 latest). **The roadmap's version assumption ("kube 0.98+") is outdated** — the crate hit a major redesign at v1.0; confirm actual API surface against v3.x docs when implementing T02. The `k8s-openapi` API for Pod, Secret, Volume, ResourceRequirements is stable and documented.

## Recommendation

Add `kube` (default features, no `ws` yet — that's S02) and `k8s-openapi` (feature `v1_32`) to `smelt-core/Cargo.toml`. Define `KubernetesConfig` in `manifest.rs` next to `ForgeConfig` pattern. Implement `generate_pod_spec()` in a new `crates/smelt-core/src/k8s.rs` module. Add `KubernetesProvider` as a stub struct with a `todo!()` impl of `RuntimeProvider`. Expose `pub mod k8s` from `lib.rs`. All existing tests must continue to pass — the runtime allowlist change in `validate()` is the only place a regression could slip in.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| K8s typed resource structs (Pod, Secret, Volume, ResourceRequirements) | `k8s-openapi` crate | Auto-generated from OpenAPI spec; matches API server exactly; `serde` derives included |
| SSH Secret `defaultMode` | `k8s-openapi::api::core::v1::SecretVolumeSource.default_mode: Option<i32>` | Must be set to `256` (0o400); if hand-rolled you forget this and SSH rejects the key |
| Kubernetes quantity strings ("100m", "256Mi") | Let through as `Option<String>` — K8s API validates at apply time | No Rust crate for quantity parsing is worth the dep; format validation is deferred to S02 per boundary map |

## Existing Code and Patterns

- `crates/smelt-core/src/manifest.rs` — **central change file**. `ForgeConfig` pattern (line ~30) shows exactly how to add an optional section with `deny_unknown_fields`: derive `Deserialize`, annotate with `#[serde(deny_unknown_fields)]`, add `#[serde(default)] pub kubernetes: Option<KubernetesConfig>` to `JobManifest`. The `validate()` function (line ~168) shows how to extend the VALID_RUNTIMES slice and add cross-field guards (services-require-compose pattern applies directly to kubernetes-requires-kubernetes-block).
- `crates/smelt-core/src/compose.rs` — `generate_compose_file()` is the reference for the pod spec generator's signature and return contract. `ComposeProjectState` (private struct, `Arc<Mutex<HashMap<ContainerId, _>>>`) is the reference for `PodState` in S02 — don't implement it in S01 (stub only).
- `crates/smelt-core/src/forge.rs` — unconditional type exports + `#[cfg(feature = "forge")]` gating pattern (D055). `KubernetesProvider` and `k8s` module are NOT feature-gated per the boundary map — they're always compiled.
- `crates/smelt-core/src/lib.rs` — wire `pub mod k8s` and `pub use k8s::KubernetesProvider` here following `pub use compose::ComposeProvider` pattern; `#[deny(missing_docs)]` is enforced, so all new public items need doc comments.
- `crates/smelt-cli/src/commands/run.rs` — `print_execution_plan()` (line ~519) is where `── Kubernetes ──` section goes in S04. The `AnyProvider` enum (line ~38) is where `Kubernetes(KubernetesProvider)` variant goes in S04. Do NOT touch these in S01.
- `crates/smelt-cli/tests/dry_run.rs` — pattern for `── Compose Services ──` test shows exactly how to add `── Kubernetes ──` dry-run test in S04. S01 adds a dry-run test only if validation passes for a kubernetes manifest (which it will once `validate()` accepts `runtime = "kubernetes"`).
- `examples/job-manifest-compose.toml` — format reference for `examples/job-manifest-k8s.toml` that S01 creates (no `[[services]]` equivalent, but same structure otherwise).

## Constraints

- `deny_unknown_fields` on all manifest structs (D017) — `KubernetesConfig` must have this; every field must be explicitly declared.
- `#[deny(missing_docs)]` enforced in `lib.rs` — every `pub` item in `k8s.rs` and every new field on `KubernetesConfig` needs a doc comment.
- VALID_RUNTIMES in `validate()` currently includes only `"docker"` and `"compose"`. Test `test_validate_runtime_unknown_rejected` passes `"kubernetes"` and asserts it IS rejected. **This test must be flipped/updated** when `"kubernetes"` is added to the allowlist — failing to update it causes a false-pass after the change.
- Validation cross-guards: if `runtime == "kubernetes"` then `kubernetes` block must be present (with non-empty `namespace` and `ssh_key_env`); if `runtime != "kubernetes"` and `kubernetes` block is present, that's also an error. Both directions must be tested (D018 collect-all-errors).
- `generate_pod_spec()` is a pure synchronous function — no async, no network. Takes `manifest: &JobManifest`, `job_name: &str`, `ssh_private_key: &str`. Returns `crate::Result<Pod>`. Must fail if `manifest.kubernetes` is `None` (misuse guard).
- `kube` dependency: add without `ws` feature for S01 (only `k8s-openapi` types are needed in S01; the client and exec are S02). If the `kube` crate stub type for `KubernetesProvider` needs `kube::Client`, that's fine to import even without the ws feature.
- `k8s-openapi` feature flag: must specify a k8s API version (e.g. `features = ["v1_32"]`) — the crate requires exactly one version feature to compile.

## Common Pitfalls

- **`defaultMode` must be `Some(256)` (0o400 in decimal)** — K8s Secrets mount with 0o444 by default; SSH clients reject anything not 0o400. The field is `Option<i32>` in `k8s-openapi`. Forgetting this compiles fine but breaks every SSH operation in S02.
- **`test_validate_runtime_unknown_rejected` expects `"kubernetes"` to FAIL** — it currently tests that `runtime = "kubernetes"` is rejected. After adding "kubernetes" to VALID_RUNTIMES, this test will pass for the wrong reason unless you update the assertion to a different unknown value (e.g. "podman").
- **`deny_unknown_fields` breaks if you add a field to the TOML schema without adding it to the struct** — always add both the TOML key and the Rust field in the same commit.
- **k8s-openapi optional fields** — many `Pod` fields are `Option<T>`. `Pod::metadata` is `ObjectMeta` (non-optional in the constructor sense, but the struct fields are `Option<String>`). Use `ObjectMeta { name: Some("...".into()), namespace: Some("...".into()), ..Default::default() }` to avoid missing 30+ None fields.
- **`kube` crate v3.x vs "v0.98+"** — the M005 roadmap and context were written before the kube crate hit v1.0+. As of 2026-03 the latest is v3.1.0. The exec/attach API may have changed between v0.98 and v3.x. This does not affect S01 (no exec in S01) but the S02 plan must be validated against v3.x docs.
- **`cargo test --workspace` runtime guard** — the new `"kubernetes"` runtime value is allowed in `validate()` but no test should try to *actually provision* with it (that requires kind). All S01 tests must be unit tests against `validate()` and `generate_pod_spec()`.

## Open Risks

- The `k8s-openapi` `SecretVolumeSource.default_mode` field type is `Option<i32>` — verify this is `i32` (not `u32` or `i64`) to avoid a compile error. Low risk — but worth checking before writing the code.
- Pod spec snapshot tests: `k8s-openapi` structs don't implement `Display`; snapshot format must use `serde_json::to_string_pretty(&pod)` or `serde_yaml::to_string(&pod)`. Choose `serde_json` (already a dep) for deterministic output.
- `KubernetesProvider` stub must implement all 5 `RuntimeProvider` trait methods (`provision`, `exec`, `exec_streaming`, `collect`, `teardown`) even as stubs. All must be `todo!()` or return `Err(SmeltError::provider("...", "not implemented"))` — the former is simpler and clearly marks unimplemented S02 work.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust / k8s-openapi | none found | none found |
| kube crate | none found | none found |

## Sources

- Existing codebase: `manifest.rs`, `compose.rs`, `lib.rs`, `run.rs`, `dry_run.rs` (direct read — HIGH confidence)
- kube crate latest version: v3.1.0 on crates.io (verified via API — HIGH confidence)
- k8s-openapi latest version: v0.27.1 on crates.io (verified via API — HIGH confidence)
- k8s-openapi `defaultMode` value: 256 decimal = 0o400 octal (K8s API convention, cross-referenced with M005-CONTEXT.md — HIGH confidence)
