---
id: T01
parent: S01
milestone: M004
provides:
  - "`indexmap` as explicit workspace dependency (v2, serde feature)"
  - "`ComposeService` struct with `name`, `image`, and serde-flatten `extra: IndexMap<String, toml::Value>`"
  - "`JobManifest.services: Vec<ComposeService>` field with `#[serde(default)]`"
  - "Backward-compatible parse: existing manifests parse with `services: vec![]`"
key_files:
  - Cargo.toml
  - crates/smelt-core/Cargo.toml
  - crates/smelt-core/src/manifest.rs
key_decisions:
  - "D073 applied: ComposeService has NO `#[serde(deny_unknown_fields)]` — intentional passthrough for unknown Compose keys via `#[serde(flatten)] extra: IndexMap<String, toml::Value>`"
patterns_established:
  - "Compose service fields follow the same serde patterns as other JobManifest structs; extra keys captured via indexmap flatten"
observability_surfaces:
  - "grep -n 'ComposeService\\|services:' crates/smelt-core/src/manifest.rs"
duration: 5min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T01: Add ComposeService struct, indexmap dependency, and services field

**Added `ComposeService` passthrough struct and `services: Vec<ComposeService>` field to `JobManifest`, with `indexmap` v2 as an explicit workspace dep — all 121 existing tests pass.**

## What Happened

Added `indexmap = { version = "2", features = ["serde"] }` to root `[workspace.dependencies]` and `indexmap.workspace = true` to `crates/smelt-core/Cargo.toml`. Imported `IndexMap` in `manifest.rs`.

Defined `ComposeService` struct (placed after `MergeConfig`, before `ValidationErrors`) with:
- `pub name: String` — service name
- `pub image: String` — container image reference
- `#[serde(flatten)] pub extra: IndexMap<String, toml::Value>` — all remaining Compose keys

Per D073, `ComposeService` deliberately omits `#[serde(deny_unknown_fields)]` so it acts as a passthrough for arbitrary Compose keys.

Added `#[serde(default)] pub services: Vec<ComposeService>` to `JobManifest` after the `forge` field. Because `JobManifest` has `#[serde(deny_unknown_fields)]`, `services` must be declared there — the `default` attribute means existing manifests without `[[services]]` parse successfully with an empty vec.

## Verification

```
cargo build -p smelt-core   → Finished (0 errors, 0 warnings)
cargo test -p smelt-core --lib 2>&1 | tail -3
  → test result: ok. 121 passed; 0 failed
```

## Diagnostics

`grep -n 'ComposeService\|services:' crates/smelt-core/src/manifest.rs` shows struct definition and field placement.

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `Cargo.toml` — added `indexmap = { version = "2", features = ["serde"] }` to `[workspace.dependencies]`
- `crates/smelt-core/Cargo.toml` — added `indexmap.workspace = true` to `[dependencies]`
- `crates/smelt-core/src/manifest.rs` — added `use indexmap::IndexMap;` import; `ComposeService` struct; `services` field on `JobManifest`
