---
estimated_steps: 5
estimated_files: 3
---

# T03: Instrument Mesh and Gossip executors with tracing spans

**Slice:** S03 — Orchestration span instrumentation
**Milestone:** M009

## Description

Add tracing spans to `run_mesh()` in `mesh.rs` and `run_gossip()` in `gossip.rs` following the same cross-thread pattern established in T02 for the DAG executor. Also add spans to the routing thread (mesh) and coordinator thread (gossip). Verify all 5 orchestrate_spans tests pass and run `just ready` for full workspace validation.

## Steps

1. In `mesh.rs`, wrap the body of `run_mesh()` in `info_span!("orchestrate::mesh", session_count = session_count, mode = "mesh").in_scope(|| { ... })`. Capture `let parent_span = Span::current();` before `thread::scope`. Clone `parent_span` into each session worker closure and wrap the worker body in `parent_span.in_scope(|| { let _s = info_span!("orchestrate::mesh::session", session_name = %name).entered(); ... })`. Add `info_span!("orchestrate::mesh::routing").in_scope(|| { ... })` around the routing thread body.
2. In `gossip.rs`, wrap the body of `run_gossip()` in `info_span!("orchestrate::gossip", session_count = session_count, mode = "gossip").in_scope(|| { ... })`. Capture `let parent_span = Span::current();` before `thread::scope`. Clone `parent_span` into each session worker closure and wrap the worker body in `parent_span.in_scope(|| { let _s = info_span!("orchestrate::gossip::session", session_name = %name).entered(); ... })`. Add `info_span!("orchestrate::gossip::coordinator").in_scope(|| { ... })` around the coordinator thread body.
3. Add `use tracing::{info_span, Span};` to both files (mesh.rs already has `tracing::info/warn/debug` — add the specific imports for span types).
4. Run `cargo test -p assay-core --test orchestrate_spans` — all 5 tests should pass. Run `cargo test -p assay-core --lib` to verify existing mesh/gossip module tests pass.
5. Run `cargo fmt --all -- --check`, `cargo clippy -p assay-core --all-targets -- -D warnings`, and `just ready` for full workspace validation.

## Must-Haves

- [ ] `run_mesh()` emits `orchestrate::mesh` root span with `session_count` and `mode` fields
- [ ] Per-session worker spans `orchestrate::mesh::session` with `session_name` field, parented to root via cross-thread `Span::current()` capture
- [ ] Mesh routing thread wrapped in `orchestrate::mesh::routing` span
- [ ] `run_gossip()` emits `orchestrate::gossip` root span with `session_count` and `mode` fields
- [ ] Per-session worker spans `orchestrate::gossip::session` with `session_name` field, parented to root via cross-thread `Span::current()` capture
- [ ] Gossip coordinator thread wrapped in `orchestrate::gossip::coordinator` span
- [ ] All 5 orchestrate_spans tests pass
- [ ] `just ready` green (full workspace)

## Verification

- `cargo test -p assay-core --test orchestrate_spans` — all 5 tests pass
- `cargo test -p assay-core --lib` — all existing tests pass
- `cargo fmt --all -- --check` — clean
- `cargo clippy -p assay-core --all-targets -- -D warnings` — clean
- `just ready` — full workspace green

## Observability Impact

- Signals added/changed: `orchestrate::mesh`, `orchestrate::mesh::session`, `orchestrate::mesh::routing`, `orchestrate::gossip`, `orchestrate::gossip::session`, `orchestrate::gossip::coordinator` spans
- How a future agent inspects this: `RUST_LOG=assay_core::orchestrate=debug` shows full orchestration span tree for any mode; `cargo test -p assay-core --test orchestrate_spans` checks span contract for all modes
- Failure state exposed: Mesh/Gossip session failures now appear within their named session span; routing/coordinator thread issues appear within their dedicated spans

## Inputs

- `crates/assay-core/tests/orchestrate_spans.rs` — T01 tests (Mesh + Gossip tests still red from T01)
- `crates/assay-core/src/orchestrate/executor.rs` — T02 pattern reference for cross-thread span parenting
- S03-RESEARCH.md — Mesh uses routing thread + session workers; Gossip uses coordinator thread (mpsc) + session workers

## Expected Output

- `crates/assay-core/src/orchestrate/mesh.rs` — instrumented with root span + per-session + routing thread spans
- `crates/assay-core/src/orchestrate/gossip.rs` — instrumented with root span + per-session + coordinator thread spans
- All 5 orchestrate_spans tests green, `just ready` green
