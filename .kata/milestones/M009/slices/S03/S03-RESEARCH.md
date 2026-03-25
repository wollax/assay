# S03: Orchestration span instrumentation — Research

**Date:** 2026-03-24
**Domain:** Rust tracing spans across threaded orchestration executors
**Confidence:** HIGH

## Summary

S03 instruments the three orchestration executors (DAG, Mesh, Gossip) and the merge runner with tracing spans. The primary challenge is cross-thread span parenting: all three executors use `std::thread::scope` to spawn worker threads, and spans created inside those workers must be nested under the orchestration root span.

The `tracing` crate does NOT automatically propagate spans across thread boundaries. `Span::current()` inside a spawned thread returns a disabled span unless the parent span is explicitly passed in. The solution is to capture `Span::current()` before entering `thread::scope`, clone it into each worker closure, and use `span.in_scope(|| { ... })` or `let _enter = span.enter()` to re-enter the parent span context. This is the standard pattern documented by the tracing crate.

The instrumentation targets are well-defined: 4 public entry functions (`run_orchestrated`, `run_mesh`, `run_gossip`, `merge_completed_sessions`), per-session worker spans in each executor, merge loop spans (per-session merge, conflict resolution), and coordinator/routing thread spans in Mesh/Gossip.

## Recommendation

Follow the same pattern established in S02 (pipeline spans): `#[instrument]` on public entry functions, `info_span!(...).in_scope(|| { ... })` for internal phases. For cross-thread propagation, capture `Span::current()` before `thread::scope`, pass it to workers, and use `parent_span.in_scope(|| { ... })` to nest worker spans under the orchestration root.

Test using the same `tracing-test` + `#[traced_test]` + `logs_contain()` pattern from S02's `pipeline_spans.rs`. Use mock session runners (already established in orchestrate_integration.rs) to exercise span trees without real agent processes.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Cross-thread span parenting | `Span::current()` + `.in_scope()` pattern from `tracing` crate | Standard approach; no custom `Layer` needed |
| Test span capture | `tracing-test` 0.2 with `no-env-filter` feature (already in workspace) | Proven in S02; `#[traced_test]` + `logs_contain()` is minimal |
| Mock session runners | Existing `orchestrate_integration.rs` test patterns | Tests already use `Fn(&ManifestSession, &PipelineConfig) -> Result<PipelineResult, PipelineError>` closures |

## Existing Code and Patterns

- `crates/assay-core/src/orchestrate/executor.rs` — DAG executor, 1233 lines. `run_orchestrated()` is the main entry point. Uses `std::thread::scope` + condvar dispatch loop. Workers spawn via `scope.spawn(move || { ... })`. No tracing spans yet, no tracing imports. Per-session outcomes tracked via `Mutex<ExecutorState>`.
- `crates/assay-core/src/orchestrate/mesh.rs` — Mesh executor, 584 lines. `run_mesh()` entry point. Uses `std::thread::scope` with routing thread + session worker threads. Has `tracing::info!`/`warn!`/`debug!` events but NO spans. Bounded concurrency via semaphore pattern.
- `crates/assay-core/src/orchestrate/gossip.rs` — Gossip executor, 628 lines. `run_gossip()` entry point. Uses `std::thread::scope` with coordinator thread (mpsc-based) + worker threads. Has `tracing::info!`/`warn!`/`debug!` events but NO spans. Similar semaphore pattern.
- `crates/assay-core/src/orchestrate/merge_runner.rs` — 1103 lines. `merge_completed_sessions()` is a sequential loop (no threading). Conflict resolution via closure. No tracing events or spans.
- `crates/assay-core/src/orchestrate/conflict_resolver.rs` — 927 lines. `resolve_conflict()` spawns a subprocess. Has `tracing::info!`/`warn!` events already.
- `crates/assay-core/src/pipeline.rs` — S02 pattern reference. Uses `#[instrument(name = "pipeline::fn_name", skip(...))]` on public functions and `info_span!("stage_name", fields).in_scope(|| { ... })` for internal blocks.
- `crates/assay-core/tests/pipeline_spans.rs` — S02 test reference. 4 tests using `#[traced_test]` + `logs_contain()`. Each triggers an early failure to exercise span entry.
- `crates/assay-core/tests/orchestrate_integration.rs` — Existing orchestration tests with real git repos and mock runners. Test infrastructure reusable for span tests.

## Constraints

- **Zero-trait convention (D001)** — no `Subscriber` trait implementations or custom `Layer` types. Use standard `tracing-test` subscriber.
- **Sync core (D007)** — no async runtime. `std::thread::scope` is the threading model.
- **Cross-crate span assertion (D136)** — `tracing-test` must use `no-env-filter` feature for spans emitted from `assay_core` module path to be visible in tests compiled under a different crate.
- **tracing does NOT auto-propagate spans across threads** — `Span::current()` in a new thread returns `Span::none()` unless the parent span is explicitly passed. This is the core challenge.
- **Worker closures are `move`** — all three executors use `scope.spawn(move || { ... })`. The parent span must be cloned (it's `Clone`) and moved into the closure.
- **Existing tracing events** — Mesh, Gossip, and conflict_resolver already have `tracing::info!/warn!/debug!` events. These will automatically nest under newly added spans without changes.
- **executor.rs has no tracing imports** — Need to add `use tracing::{info_span, Span};` (or equivalent) to executor.rs.

## Common Pitfalls

- **Forgetting `Span::current()` capture before `thread::scope`** — If the root span is created via `#[instrument]` on `run_orchestrated()`, the span is active in the calling thread. Inside `thread::scope` workers, `Span::current()` returns `Span::none()`. Must capture the span before entering `scope` and pass it to workers.
- **Holding `Span::enter()` guard across await/yield points** — Not relevant here (sync code), but worth noting: `.enter()` returns a guard that must be dropped before the thread exits the scope. Use `.in_scope()` for scoped span entry to be safe.
- **Span name collisions in `logs_contain()` assertions** — `logs_contain("session")` could match any event containing "session". Use distinct span names like `orchestrate::dag::session`, `orchestrate::mesh::session`, etc. to avoid false matches in test assertions.
- **`#[instrument]` on generic functions** — `run_orchestrated<F>`, `run_mesh<F>`, `run_gossip<F>` are generic. `#[instrument]` works on generics but the `skip` attribute is important to avoid trying to `Debug`-format the closure parameter.
- **Large closure captures after adding span clones** — Each worker closure already captures session data, mutexes, and config refs. Adding a cloned `Span` is cheap (Arc-based internally), but be aware of the capture list growing.

## Open Risks

- **Test complexity for multi-threaded span trees** — Testing that worker spans are properly nested under the root span requires multi-session test setups. Can mitigate by using minimal 2-session manifests with instant mock runners.
- **`logs_contain()` is substring-based** — Can produce false positives if span names are substrings of other spans or event fields. Mitigate with distinct span name prefixes (`orchestrate::dag::`, `orchestrate::mesh::`, `orchestrate::gossip::`, `merge::`).
- **Integration test timeout** — Existing `orchestrate_integration.rs` tests are slow (~300s). New span tests should use lightweight mock runners to avoid similar timeout issues.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust tracing | none found | n/a — `tracing` is well-documented; S02 established the pattern |
| Ratatui/TUI | swiftui (installed) | not relevant to this slice |

No relevant skills to install for S03 — this is pure Rust `tracing` instrumentation following the established S02 pattern.

## Sources

- S02 implementation (`pipeline.rs`, `pipeline_spans.rs`) — authoritative pattern reference for `#[instrument]` + `info_span!` + `tracing-test` assertions
- `tracing` crate documentation — `Span::current()` captures the active span; `span.in_scope(|| { ... })` re-enters it in another context
- S01 summary — `init_tracing()` + `TracingGuard` pattern; `tracing-subscriber` layered architecture
