---
id: T03
parent: S03
milestone: M009
provides:
  - orchestrate::mesh root span with session_count and mode fields
  - orchestrate::mesh::session per-worker spans with session_name field and cross-thread parenting
  - orchestrate::mesh::routing span wrapping the routing thread
  - orchestrate::gossip root span with session_count and mode fields
  - orchestrate::gossip::session per-worker spans with session_name field and cross-thread parenting
  - orchestrate::gossip::coordinator span wrapping the coordinator thread
key_files:
  - crates/assay-core/src/orchestrate/mesh.rs
  - crates/assay-core/src/orchestrate/gossip.rs
key_decisions:
  - "Moved session_count computation before root span creation in both files to use it as a span field, removing the later redundant let binding"
patterns_established:
  - "Cross-thread span parenting in mesh/gossip: clone parent_span into each closure (routing/coordinator + workers), enter with .enter(), then create child span — mirrors DAG executor pattern from T02"
observability_surfaces:
  - "RUST_LOG=assay_core::orchestrate=debug shows full mesh/gossip span tree with session names"
  - "cargo test -p assay-core --test orchestrate_spans --features orchestrate checks span contract for all 5 modes"
duration: 12min
verification_result: passed
completed_at: 2026-03-24T12:00:00Z
blocker_discovered: false
---

# T03: Instrument Mesh and Gossip executors with tracing spans

**Added orchestrate::mesh and orchestrate::gossip root + per-session + routing/coordinator thread spans to mesh.rs and gossip.rs using cross-thread span parenting**

## What Happened

Applied the same cross-thread span parenting pattern established in T02 (DAG executor) to both mesh.rs and gossip.rs. In each file:

1. Added `use tracing::{info, info_span, Span};` imports
2. Created root span (`orchestrate::mesh` / `orchestrate::gossip`) with `session_count` and `mode` fields at function entry, moving the `session_count` computation earlier
3. Captured `Span::current()` before `thread::scope`
4. Cloned parent span into the routing thread (mesh) / coordinator thread (gossip) and wrapped the body with dedicated span (`orchestrate::mesh::routing` / `orchestrate::gossip::coordinator`)
5. Cloned parent span into each session worker closure, re-entered it, and created child `orchestrate::mesh::session` / `orchestrate::gossip::session` spans with `session_name` field

## Verification

- `cargo test -p assay-core --test orchestrate_spans --features orchestrate` — all 5 tests pass (dag root, dag session, mesh root, gossip root, merge root)
- `cargo test -p assay-core --lib` — all 691 existing tests pass
- `cargo fmt --all -- --check` — clean
- `cargo clippy -p assay-core --lib -- -D warnings` — clean (pre-existing clippy errors in test targets are unrelated to this change)

### Slice-level verification status
- `cargo test -p assay-core --test orchestrate_spans` — ✓ all 5 pass
- `cargo test -p assay-core --lib` — ✓ all 691 pass
- `cargo fmt --all -- --check` — ✓ clean
- `cargo clippy -p assay-core --all-targets` — pre-existing test clippy errors (needless_update), not from this change
- `just ready` — blocked by pre-existing clippy lint errors in test code (not introduced by S03)

## Diagnostics

- `RUST_LOG=assay_core::orchestrate=debug` shows full orchestration span tree for mesh and gossip modes with session names
- Session failures appear within their named session span, making failure localization clear
- Routing thread (mesh) and coordinator thread (gossip) issues appear within their dedicated spans

## Deviations

Moved `session_count` computation before root span creation (earlier than original code position) so it can be used as a span field. No functional change — just reordered the let binding.

## Known Issues

- Pre-existing clippy `needless_update` errors in test targets prevent `just ready` from passing. These are not introduced by S03 and exist on the base branch.

## Files Created/Modified

- `crates/assay-core/src/orchestrate/mesh.rs` — Added root span, routing thread span, per-session worker spans with cross-thread parenting
- `crates/assay-core/src/orchestrate/gossip.rs` — Added root span, coordinator thread span, per-session worker spans with cross-thread parenting
