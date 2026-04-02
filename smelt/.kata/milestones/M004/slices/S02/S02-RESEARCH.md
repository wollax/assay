# S02: Compose File Generation — Research

**Date:** 2026-03-21
**Domain:** YAML code generation, serde type mapping
**Confidence:** HIGH

## Summary

S02 implements `generate_compose_file()` — a pure function that takes a `JobManifest`, a project name, and resolved credential env vars, and returns a valid Docker Compose YAML string. The YAML includes all `[[services]]` entries passed through from the manifest, plus an injected `smelt-agent` service and a named network.

The primary risk from the roadmap — **TOML → YAML type fidelity** — is fully retired by this research. Experimental testing confirms that `serde_yaml::to_value(&toml::Value)` preserves all relevant types correctly: integers stay integers, booleans stay booleans, arrays become YAML sequences, nested tables become mappings. There are no coercion surprises.

The main implementation constraint to know upfront: `toml::Value::Table` (TOML's internal map type) is a `BTreeMap` — so nested table keys (e.g., `healthcheck` sub-keys) come out in alphabetical order regardless of TOML source order. The `IndexMap<String, toml::Value>` in `ComposeService.extra` preserves top-level extra field order, but since TOML's serde flatten feeds keys in BTreeMap order, top-level extra keys are also alphabetical. Snapshot tests must reflect this.

The `serde_yaml` crate (D076) is deprecated (`0.9.34+deprecated`, dtolnay archived the repo). It still works and ships no breaking changes. This is an acceptable short-term choice; no action needed for S02, but the team should be aware.

## Recommendation

Implement `generate_compose_file()` as a `crate::Result<String>` (not bare `String`) because `resolve_repo_path()` can fail at runtime. Build the YAML via `serde_yaml::Mapping` directly — do not hand-build YAML strings. Sort the `extra_env` credential map before serializing to guarantee deterministic environment sections in snapshot tests. Place the implementation in a new `crates/smelt-core/src/compose.rs` module with `pub mod compose` in `lib.rs`.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| TOML value → YAML conversion | `serde_yaml::to_value(&toml::Value)` | Handles all type variants correctly; zero manual mapping needed |
| YAML serialization | `serde_yaml::Mapping` + `serde_yaml::to_string()` | Type-safe, insertion-order-respecting via IndexMap-compatible API |
| Repo path resolution | `crate::manifest::resolve_repo_path()` | Already in smelt-core; validates local path, rejects URLs |
| Credential env resolution | Pattern from `docker.rs` — iterate `manifest.credentials.env`, call `std::env::var()` | Same env-var-to-value pattern as DockerProvider |

## Existing Code and Patterns

- `crates/smelt-core/src/docker.rs` — `DockerProvider::provision()` builds `env: Vec<String>` from `manifest.credentials.env` by reading actual env var values. `generate_compose_file()` receives these already-resolved values via `extra_env: &HashMap<String, String>`. Follow the same read pattern in `ComposeProvider::provision()` before calling `generate_compose_file()`.
- `crates/smelt-core/src/manifest.rs` — `ComposeService { name, image, extra: IndexMap<String, toml::Value> }` and `VALID_COMPOSE_MANIFEST` constant. This constant covers all four TOML extra-field types and is the correct snapshot test input.
- `crates/smelt-core/src/lib.rs` — add `pub mod compose;` here and re-export `ComposeProvider` from `use crate::compose::ComposeProvider`.
- `crates/smelt-core/src/error.rs` — `SmeltError::provider()` / `SmeltError::Provider` for error wrapping.

## YAML Structure

The generated YAML has this shape (no `version:` key — Compose v2+ format per scope decision):

```yaml
services:
  <svc-name>:              # from [[services]] name
    image: <image>         # from [[services]] image
    <extra-key>: <val>     # alphabetical order (BTreeMap from TOML flatten)
    ...
  smelt-agent:
    image: <environment.image>
    volumes:
    - <repo_path>:/workspace
    environment:
      <CRED_KEY>: <val>    # sorted alphabetically for determinism
    depends_on:            # omitted when services is empty
    - <svc-name-1>
    - <svc-name-2>
    networks:
    - smelt-<project_name>
networks:
  smelt-<project_name>: {}
```

**Service insertion order**: user services first (in manifest order), then `smelt-agent` last. Within each service, `image` is inserted first explicitly, then extra fields follow in BTreeMap/alphabetical order.

**Network entry**: use empty `serde_yaml::Mapping::new()` (renders as `{}`) — cleaner than `null`.

**`depends_on`**: only present when `manifest.services` is non-empty. When services is empty, omit the key entirely (don't emit `depends_on: []`).

**`environment`**: use a YAML mapping (key: value) not a sequence (`KEY=VALUE` strings). Iterate `extra_env` via a `BTreeMap` iterator for deterministic key order.

## Constraints

- `serde_yaml = "0.9"` must be added to `smelt-core/Cargo.toml` as a **production dep** (not dev-only). D076 is firm — `generate_compose_file()` runs in the normal `smelt run` path.
- `serde_yaml` is NOT currently in `workspace.dependencies` and should not be added there — only smelt-core needs it.
- `generate_compose_file()` return type must be `crate::Result<String>`, not bare `String`, because `resolve_repo_path()` can fail.
- `#![deny(missing_docs)]` is enforced on smelt-core. All public items in `compose.rs` need `///` doc comments.
- D019 (RPITIT): `ComposeProvider`'s `RuntimeProvider` impl comes in S03. In S02, only define the struct and `generate_compose_file()`.
- D073 (no `deny_unknown_fields` on `ComposeService`): do not add it. Passthrough is the design.
- D074 (credentials to smelt-agent only): `extra_env` goes to `smelt-agent` service; never injected into user services.

## Common Pitfalls

- **`extra_env: HashMap<String, String>` is non-deterministic** — iterating a `HashMap` directly produces different key orders across runs. Always convert to `BTreeMap` before building the environment mapping. Without this, snapshot tests fail intermittently.

- **`toml::Value::Table` key order is alphabetical (BTreeMap), not insertion order** — TOML's internal map is `BTreeMap`. When `[[services]]` entries contain nested tables (e.g., `[healthcheck]`), their sub-keys appear alphabetically in the YAML output, regardless of order in the TOML file. Snapshot tests must use the alphabetically-sorted expected string.

- **Top-level extra fields are also alphabetical** — `ComposeService.extra` is `IndexMap<String, toml::Value>`, which preserves insertion order. However, TOML's serde flatten deserializer feeds keys in BTreeMap order when the source is a TOML table. Result: extra fields come out alphabetical, not in TOML file order. The test in `VALID_COMPOSE_MANIFEST` has `port`, `restart`, `command`, `tag` in TOML source order — but `extra.keys()` will be `["command", "port", "restart", "tag"]` (alphabetical).

- **`serde_yaml::Mapping` vs `IndexMap`** — `serde_yaml::Mapping` is internally ordered but uses its own key type (`serde_yaml::Value`). Build services by inserting `image` first, then iterating `extra`. This produces consistent `image`-first output regardless of the extra field names.

- **`smelt-agent` service name** — hyphens in YAML keys don't need quoting; `smelt-agent:` serializes correctly as a bare key.

- **Empty services case** — `runtime = "compose"` with zero `[[services]]` is valid (per validate()). In this case: user services section is empty, `smelt-agent` has no `depends_on`, and no `networks` entry is needed on the agent unless a network is still desired. Per scope: still inject the network even with no services.

## Open Risks

- **serde_yaml deprecation** — `0.9.34+deprecated` is dtolnay's final release. It works and has no security issues known at time of research, but the ecosystem is moving toward `serde_yml` (maintained by The YAML Organization) or `serde_yaml_ng`. If smelt-core adds YAML for other purposes or if a vulnerability is found, migration will be needed. Not a S02 blocker.

- **`serde_yaml::to_string()` panics on unusual toml::Value variants** — `toml::Value::Datetime` would serialize to a YAML string representation (not YAML timestamp). This is unlikely in Compose service definitions but untested. If users pass TOML datetime values in service extra fields, the YAML may surprise them. Not a blocker for the S02 scope, which targets known Compose service patterns.

## Implementation Checklist

For S02 specifically:

1. Add `serde_yaml = "0.9"` to `[dependencies]` in `crates/smelt-core/Cargo.toml`
2. Create `crates/smelt-core/src/compose.rs` with:
   - `pub struct ComposeProvider {}` (stub — RuntimeProvider impl deferred to S03)
   - `pub fn generate_compose_file(manifest: &JobManifest, project_name: &str, extra_env: &HashMap<String, String>) -> crate::Result<String>`
   - Private helper `fn toml_to_yaml(v: &toml::Value) -> serde_yaml::Value`
3. Add `pub mod compose;` to `lib.rs`; re-export `ComposeProvider` from top-level
4. Tests in `compose.rs` (under `#[cfg(test)]`):
   - `test_generate_compose_postgres_only` — snapshot test against exact YAML string
   - `test_generate_compose_postgres_and_redis` — snapshot test with two services
   - `test_generate_compose_empty_services` — agent-only compose (no user services, no `depends_on`)
   - `test_generate_compose_type_fidelity` — integer, boolean, array extra fields produce correct YAML types
   - `test_generate_compose_nested_healthcheck` — nested TOML table in extra fields (alphabetical key order)
   - `test_generate_compose_empty_extra_env` — agent with no credentials env

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| serde_yaml | none relevant | none found |
| Docker Compose YAML | none relevant | none found |

## Sources

- TOML→YAML type mapping: experimental testing with `serde_yaml = "0.9"` + `toml = "1"` + `indexmap = "2"` in a scratch project (confirmed via direct Rust compilation and execution)
- Key ordering behavior: `toml::value::Table` is `BTreeMap` (alphabetical) confirmed by deserializing with `#[serde(flatten)] extra: IndexMap<String, toml::Value>` and printing key order
- `serde_yaml` deprecation status: `cargo search serde_yaml` output showing `"0.9.34+deprecated"` version tag
- Docker Compose v2 format (no `version:` key): existing M004-CONTEXT.md scope decision and D076
- `smelt-core` module patterns: reading `lib.rs`, `docker.rs`, `manifest.rs` directly
