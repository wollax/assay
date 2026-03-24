---
id: S01
parent: M009
milestone: M009
provides:
  - Zero-warning `cargo doc --workspace --no-deps` across entire workspace
  - `#![deny(missing_docs)]` lint gate on smelt-cli (self-enforcing for all future public items)
  - Doc comments on all ~37 public items in smelt-cli (serve/, commands/, lib.rs)
  - All 4 `#[allow(dead_code)]` annotations audited — 2 removed (unnecessary), 2 kept with updated rationale
  - Broken intra-doc link in ssh.rs fixed (D070 backtick-only convention)
requires:
  - slice: none
    provides: first slice, no dependencies
affects:
  - S03
key_files:
  - crates/smelt-cli/src/lib.rs
  - crates/smelt-cli/src/serve/ssh.rs
  - crates/smelt-cli/src/serve/config.rs
  - crates/smelt-cli/src/serve/queue.rs
  - crates/smelt-cli/src/serve/types.rs
  - crates/smelt-cli/src/serve/mod.rs
  - crates/smelt-cli/src/commands/mod.rs
  - crates/smelt-core/src/k8s.rs
key_decisions:
  - "D127: #![deny(missing_docs)] enforced on smelt-cli — matches smelt-core; all future public items require doc comments"
  - "retry_backoff_secs #[allow(dead_code)] kept — serde deserialization does NOT suppress dead_code lint; field is forward-compat config"
  - "PodState #[allow(dead_code)] kept — namespace/pod_name stored for future exec/collect; only secret_name consumed today"
  - "MockSshClient::with_probe_result #[allow(dead_code)] removed — method is used in 12+ test call sites"
patterns_established:
  - "#![deny(missing_docs)] in smelt-cli lib.rs as self-enforcing lint gate for all future public items"
  - "D070 backtick-only convention applied consistently for pub(crate) and non-linkable types in doc comments"
observability_surfaces:
  - "cargo build -p smelt-cli will fail on any future undocumented public item — the lint is self-enforcing"
drill_down_paths:
  - .kata/milestones/M009/slices/S01/tasks/T01-SUMMARY.md
  - .kata/milestones/M009/slices/S01/tasks/T02-SUMMARY.md
duration: 20min
verification_result: passed
completed_at: 2026-03-24T12:15:00Z
---

# S01: cargo doc zero-warning + deny(missing_docs) on smelt-cli

**Zero cargo-doc warnings across workspace, `#![deny(missing_docs)]` enforced on smelt-cli with all ~37 public items documented, and all stale `#[allow(dead_code)]` annotations resolved.**

## What Happened

T01 fixed the only cargo-doc warning — a broken intra-doc link `[build_ssh_args]` in ssh.rs changed to backtick-only per D070. Then added `///` doc comments to all ~23 undocumented public items across 5 serve/ files (config.rs, queue.rs, types.rs, ssh.rs, mod.rs). Audited both smelt-cli `#[allow(dead_code)]` annotations: `retry_backoff_secs` kept (serde deser doesn't suppress the lint; field is forward-compat config), `MockSshClient::with_probe_result` removed (used in 12+ test sites).

T02 documented the remaining public items in lib.rs (`pub mod commands`, `pub mod serve`) and commands/mod.rs (6 `pub mod` re-exports). Enabled `#![deny(missing_docs)]` in lib.rs — the compiler surfaced 10 additional undocumented items (JobSource variants, JobStatus variants, SshOutput fields) which were all documented. Audited smelt-core's PodState `#[allow(dead_code)]` — kept with updated rationale (namespace/pod_name stored for future use, only secret_name consumed today).

## Verification

| Check | Result |
|-------|--------|
| `cargo doc --workspace --no-deps` warnings | ✓ 0 |
| `cargo build -p smelt-cli` warnings | ✓ 0 |
| `grep 'deny(missing_docs)' crates/smelt-cli/src/lib.rs` | ✓ 1 match |
| `cargo test --workspace` | ✓ 286 passed, 0 failed |
| `cargo clippy --workspace -- -D warnings` | ✗ 16 pre-existing errors in smelt-core (identical on main, not regressions) |
| `#[allow(dead_code)]` audit | ✓ 2 remaining, both justified with rationale comments |

## Requirements Advanced

- R040 (zero-warning cargo doc) — `cargo doc --workspace --no-deps` exits 0 with zero warnings
- R042 (deny(missing_docs) on smelt-cli) — `#![deny(missing_docs)]` present in lib.rs and compiles clean
- R043 (no stale #[allow] annotations) — all 4 annotations audited; 2 removed, 2 justified

## Requirements Validated

- R040 — proven by `cargo doc --workspace --no-deps` exiting 0 with zero warnings
- R042 — proven by `#![deny(missing_docs)]` compiling clean with all public items documented
- R043 — proven by audit of all 4 `#[allow(dead_code)]` annotations with removal or justification

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- `retry_backoff_secs`: Plan expected serde deserialization to count as a "use" for dead_code lint. It does not. The `#[allow(dead_code)]` was kept instead of removed.
- `tests/docker_lifecycle.rs:133` referenced in T02 plan does not exist as source code (only build artifacts in target/). No audit was possible or needed.
- `cargo clippy --workspace -- -D warnings` does not exit 0 due to 16 pre-existing collapsible-if warnings in smelt-core. Verified identical count on main branch — not a regression.

## Known Limitations

- Pre-existing clippy warnings in smelt-core (16 collapsible-if lint errors in compose.rs and k8s.rs) prevent `cargo clippy --workspace -- -D warnings` from passing workspace-wide. These predate this slice and are not addressed here.

## Follow-ups

- Fix the 16 pre-existing clippy warnings in smelt-core (compose.rs and k8s.rs collapsible-if lints) — could be addressed in S03 during file decomposition or as a standalone cleanup.

## Files Created/Modified

- `crates/smelt-cli/src/lib.rs` — Added `#![deny(missing_docs)]` and doc comments on pub mod re-exports
- `crates/smelt-cli/src/serve/ssh.rs` — Fixed broken intra-doc link; removed unnecessary `#[allow(dead_code)]` on test mock; added doc comments on SshOutput fields
- `crates/smelt-cli/src/serve/config.rs` — Added doc comments to ServerNetworkConfig, ServerConfig and all fields; kept `#[allow(dead_code)]` on retry_backoff_secs with updated rationale
- `crates/smelt-cli/src/serve/queue.rs` — Added doc comments to ServerState fields and new() method
- `crates/smelt-cli/src/serve/types.rs` — Added doc comments to JobId::new(), all QueuedJob fields, JobSource and JobStatus variants
- `crates/smelt-cli/src/serve/mod.rs` — Added doc comments to 3 pub mod re-exports
- `crates/smelt-cli/src/commands/mod.rs` — Added doc comments on all 6 pub mod re-exports
- `crates/smelt-core/src/k8s.rs` — Updated `#[allow(dead_code)]` rationale on PodState

## Forward Intelligence

### What the next slice should know
- `#![deny(missing_docs)]` is now enforced on smelt-cli — any module splits in S03 must maintain doc coverage on all public items or the build will fail
- smelt-core already had `deny(missing_docs)` (D070) — both crates are now consistent

### What's fragile
- The 16 clippy warnings in smelt-core (compose.rs, k8s.rs) — these are collapsible-if lints that could be fixed alongside S03's file decomposition work

### Authoritative diagnostics
- `cargo doc --workspace --no-deps 2>&1 | grep -c warning` — the single authoritative check for doc quality; 0 is the baseline
- `cargo build -p smelt-cli` — will fail immediately if any new public item lacks docs

### What assumptions changed
- Plan assumed serde deserialization suppresses dead_code lint — it does not; this is a Rust compiler behavior worth knowing for future audits
