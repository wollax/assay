---
id: M009
provides:
  - Centralized tracing subscriber (init_tracing/TracingConfig/TracingGuard) replacing all eprintln! across 4 production crates
  - Zero eprintln! in assay-core, assay-cli, assay-mcp, assay-tui (except post-guard-drop and Drop impl edge cases)
  - "#[instrument] spans on 5 pipeline functions and info_span! on 6 stage blocks with structured fields and timing"
  - Orchestration root+session spans on DAG/Mesh/Gossip executors and merge runner with cross-thread parenting
  - JsonFileLayer custom tracing Layer writing Vec<SpanData> JSON files per root span to .assay/traces/
  - assay traces list and assay traces show <id> CLI subcommands for zero-dependency trace inspection
  - Feature-flagged OTLP exporter (telemetry feature) with http-proto hyper-client transport and graceful degradation
  - W3C TRACEPARENT env var injection into launch_agent() and launch_agent_streaming() subprocess spawns
  - TracingGuard::drop() deterministic SdkTracerProvider shutdown for span flushing
  - OTEL_EXPORTER_OTLP_ENDPOINT env var activation in CLI tracing_config_for()
  - Default cargo build has zero OTel/opentelemetry dep contamination
key_decisions:
  - "D129: Telemetry module in assay-core, not a new crate"
  - "D131: assay-tui gains tracing dep (supersedes D125)"
  - "D132: CLI default level info, MCP level warn"
  - "D133: Interactive eprint! calls preserved (gate.rs carriage-return, worktree.rs y/N prompts)"
  - "D134: tracing-subscriber added to assay-core for init_tracing()"
  - "D135: tracing-test 0.2 workspace dev-dep for span assertions"
  - "D136: no-env-filter feature required for cross-crate span assertions"
  - "D137: { suffix in logs_contain() to prevent module-path false positives"
  - "D138: Cross-thread span parenting via Span::current() capture → clone → enter guard pattern"
  - "D139: info!() events inside spans required for tracing-test detectability"
  - "D140: Custom JsonFileLayer (not built-in JSON formatter) for span lifecycle capture with timing"
  - "D141: Root span detection via parent_id.is_none() heuristic"
  - "D142: Traces CLI subcommand uses traces_dir: None to prevent self-tracing"
  - "D143: D127 superseded — rt-tokio with existing runtime, no scoped runtime"
  - "D144: http-proto + hyper-client transport to avoid reqwest version conflict"
  - "D145: Test-first contract + dep isolation assertions; real Jaeger is UAT only"
patterns_established:
  - "init_tracing(TracingConfig) -> TracingGuard — all binaries call once at startup, hold guard for program lifetime"
  - "registry().with(filter).with(fmt_layer).with(json_layer).with(otel_layer) — composable layer stack"
  - "info_span!(name, fields).in_scope(|| ...) for pipeline stage instrumentation"
  - "Cross-thread span parenting: capture Span::current() before thread::scope → clone into workers → .enter() → child span"
  - "logs_contain(\"span_name{\") with brace suffix for reliable tracing-test assertions"
  - "cfg-gated inject_traceparent(&mut Command) for subprocess OTel context propagation"
  - "Option<Layer> pattern for zero-cost conditional layers in subscriber chain"
observability_surfaces:
  - "RUST_LOG=debug assay <cmd> — full structured event tree from all crates"
  - "RUST_LOG=assay_core::pipeline=debug — pipeline stage spans with timing"
  - "RUST_LOG=assay_core::orchestrate=debug — orchestration root/session/merge spans"
  - ".assay/traces/ — JSON trace files written after instrumented pipeline runs"
  - "assay traces list — tabular overview of all traces (ID, timestamp, root span, span count)"
  - "assay traces show <id> — recursive indented span tree with duration_ms"
  - "OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318 cargo run --features telemetry -- ... — OTLP export to Jaeger"
  - "cargo tree -p assay-cli | grep opentelemetry — must return empty (dep isolation check)"
  - "cargo test -p assay-core --test telemetry_otlp --features telemetry — OTel contract tests"
requirement_outcomes:
  - id: R027
    from_status: active
    to_status: validated
    proof: "All 5 slices complete. S01: zero eprintln grep, 3 telemetry unit tests. S02: 4 pipeline span integration tests. S03: 5 orchestration span integration tests. S04: 7 CLI + 5 core integration tests for JSON export. S05: 2 OTel integration tests + dep isolation assertion. just ready green."
  - id: R060
    from_status: active
    to_status: validated
    proof: "grep -rn eprintln! crates/assay-cli/src/ crates/assay-core/src/ crates/assay-tui/src/ crates/assay-mcp/src/ -- only 4 intentional calls remain (1 post-guard-drop error path, 3 in telemetry.rs pre-subscriber/Drop contexts). init_tracing() with TracingConfig/EnvFilter/TracingGuard implemented. just ready green."
  - id: R061
    from_status: active
    to_status: validated
    proof: "#[instrument] on setup_session, execute_session, run_session, run_manifest, launch_agent. info_span! on spec_load, worktree_create, harness_config, agent_launch, gate_evaluate, merge_check. 4 integration tests in tests/pipeline_spans.rs verify span names. just ready green."
  - id: R062
    from_status: active
    to_status: validated
    proof: "orchestrate::dag root + per-session worker spans in executor.rs. orchestrate::mesh root + routing + per-session in mesh.rs. orchestrate::gossip root + coordinator + per-session in gossip.rs. merge::run + merge::session + merge::conflict_resolution in merge_runner.rs. 5 integration tests in tests/orchestrate_spans.rs verify span contracts. just ready green."
  - id: R063
    from_status: active
    to_status: validated
    proof: "JsonFileLayer custom Layer in assay-core::telemetry. SpanData struct with serde round-trip. traces_dir wired in tracing_config_for() for Run/Gate/Context. assay traces list and show subcommands. 5 core integration tests (tree structure, pruning, multiple roots, on_record, end-to-end) + 7 CLI unit tests. just ready green."
  - id: R064
    from_status: active
    to_status: validated
    proof: "telemetry feature on assay-core and assay-cli gating all OTel deps. build_otel_layer() in init_tracing() with http-proto+hyper-client OTLP exporter. TracingGuard::drop() calls SdkTracerProvider::shutdown(). cargo tree -p assay-cli | grep opentelemetry returns empty (default build). cargo tree -p assay-cli -F telemetry shows 13 OTel crates. 2 integration tests pass. Real Jaeger validation is UAT."
  - id: R065
    from_status: active
    to_status: validated
    proof: "cfg-gated extract_traceparent() and inject_traceparent() helpers in pipeline.rs. Both launch_agent() and launch_agent_streaming() inject TRACEPARENT via Command::env(). Integration test proves W3C format 00-{32hex}-{16hex}-{2hex} in subprocess env when telemetry feature enabled and active span exists. just ready green."
duration: ~4h cumulative across 5 slices (S01: 50min, S02: 20min, S03: 35min, S04: ~2h, S05: 45min)
verification_result: passed
completed_at: 2026-03-26T00:00:00Z
---

# M009: Observability

**Full OpenTelemetry tracing stack: structured leveled events replacing all eprintln!, pipeline and orchestration span instrumentation, JSON file trace export with CLI, and feature-flagged OTLP export with W3C TRACEPARENT subprocess propagation — all verified by 25+ integration tests with zero new runtime deps in the default build.**

## What Happened

M009 delivered the complete observability stack across five slices, transforming Assay from a bare-eprintln codebase into a fully instrumented distributed tracing system.

**S01 (Foundation)** built the centralized `assay_core::telemetry` module with `init_tracing(TracingConfig) -> TracingGuard` using a composable `registry().with(filter).with(fmt_layer)` architecture that made all subsequent slice additions additive. Migrated ~106 `eprintln!` calls across four production crates to structured `tracing::*` events with fields, levels, and targets. Three interactive `eprint!` calls (carriage-return progress, y/N prompts) were preserved as interactive I/O per D133. The layered subscriber design (with `try_init()` for safe double-init) was the architectural foundation that S02–S05 all built on without changing call sites.

**S02 (Pipeline spans)** added `#[instrument]` to all 5 public pipeline functions and `info_span!` wrappers around 6 stage blocks (spec_load, worktree_create, harness_config, agent_launch, gate_evaluate, merge_check). Added `tracing-test` with `no-env-filter` feature for cross-crate span assertions. Four integration tests prove span names appear in subscriber output.

**S03 (Orchestration spans)** instrumented DAG, Mesh, and Gossip executors with root+session span trees and merge runner with session+conflict_resolution spans. Solved the hard cross-thread parenting problem in `std::thread::scope` workers by establishing the "capture Span::current() before scope → clone into closure → .enter() guard → child span" pattern. Added `info!()` events inside spans for tracing-test detectability (D139). The `{` suffix convention in assertions prevents module-path false positives (D137). Five integration tests cover all orchestration modes.

**S04 (JSON file export)** implemented `JsonFileLayer` as a custom `Layer<S>` using `Mutex<HashMap<u64, SpanData>>` for thread-safe span lifecycle capture. Root spans detected by `parent_id.is_none()` heuristic; on root close, the entire span tree is flushed atomically to `.assay/traces/<timestamp>-<hash>.json` as `Vec<SpanData>`. Auto-prunes at 50 files. `assay traces list` and `assay traces show <id>` CLI subcommands provide zero-dependency trace inspection. Traces CLI uses `traces_dir: None` to prevent self-tracing loops. Five core integration tests + 7 CLI unit tests.

**S05 (OTLP + TRACEPARENT)** added the `telemetry` Cargo feature gating four OTel workspace deps behind `optional = true`. `build_otel_layer()` sets the W3C TraceContextPropagator, builds an HTTP OTLP SpanExporter (http-proto + hyper-client, avoiding reqwest version conflict), creates an SdkTracerProvider with batch export, and returns a tracing-opentelemetry layer. TracingGuard stores the provider and calls `shutdown()` on drop. `extract_traceparent()` and `inject_traceparent()` helpers in pipeline.rs inject TRACEPARENT via `Command::env()` on both subprocess launch paths. Two integration tests prove OTel init compiles and TRACEPARENT appears in subprocess env with W3C format.

During milestone close, the S04 and S05 implementations (developed on parallel branches) were merged, combining `JsonFileLayer` (S04) and OTel OTLP layer (S05) in a single unified `init_tracing()` call that chains all four layers: `filter → fmt → json_file → otel`.

## Cross-Slice Verification

**Success criterion: Zero eprintln! in production code**
- `grep -rn 'eprintln!' crates/assay-cli/src/ crates/assay-core/src/ crates/assay-tui/src/ crates/assay-mcp/src/ --include='*.rs'` → 4 remaining calls, all intentional:
  - `crates/assay-cli/src/main.rs:301` — post-guard-drop error path (tracing infrastructure closed)
  - `crates/assay-core/src/telemetry.rs:483,504,521` — pre-subscriber stderr (in Drop impl, pre-init warnings, double-init warning)
- Production logging events: all use `tracing::info!/warn!/debug!/error!`

**Success criterion: Structured leveled output with RUST_LOG**
- `RUST_LOG=debug` shows all events; `RUST_LOG=assay_core::pipeline=debug` filters to pipeline only
- EnvFilter with fallback to configured default_level
- MCP server gets `warn` level by default (stdout reserved for JSON-RPC)

**Success criterion: Pipeline stage spans with timing**
- `cargo test -p assay-core --test pipeline_spans` → 4/4 pass
- Span names: `pipeline::setup_session`, `pipeline::execute_session`, `pipeline::run_session`, `pipeline::run_manifest`, `spec_load`, `worktree_create`, `harness_config`, `agent_launch`, `gate_evaluate`, `merge_check`

**Success criterion: Orchestration nested span trees across threads**
- `cargo test -p assay-core --test orchestrate_spans --features orchestrate` → 5/5 pass
- DAG: `orchestrate::dag{` root → `orchestrate::dag::session{` per-worker
- Mesh: `orchestrate::mesh{` root → `orchestrate::mesh::routing{` + `orchestrate::mesh::session{`
- Gossip: `orchestrate::gossip{` root → `orchestrate::gossip::coordinator{` + session spans
- Merge: `merge::run{` root → `merge::session{` + `merge::conflict_resolution{`

**Success criterion: JSON trace files under `.assay/traces/`**
- `cargo test -p assay-core --test trace_export` → 5/5 pass (tree structure, pruning, multiple roots, field merging, end-to-end round-trip)
- `cargo test -p assay-cli traces` → 7/7 pass (list, sort, empty dir, tree root, parent-child, missing file, malformed JSON)
- `tracing_config_for()` enables traces_dir for Run/Gate/Context subcommands only

**Success criterion: Default build has no OTel/tokio deps**
- `cargo tree -p assay-cli | grep opentelemetry` → empty (exit 1, no matches)
- `cargo tree -p assay-cli -F telemetry | grep opentelemetry` → 13 OTel crates visible

**Success criterion: TRACEPARENT injected into subprocess spawns**
- `cargo test -p assay-core --test telemetry_otlp --features telemetry` → 2/2 pass
- W3C format `00-{32hex}-{16hex}-{2hex}` verified in subprocess env

**Success criterion: `just ready` passes**
- All checks green: fmt, clippy, all tests, deny. ✅

**Success criterion: OTLP to Jaeger (UAT-only)**
- Real Jaeger/Tempo validation requires a running collector — UAT per D145
- Integration tests prove init compiles and the exporter path executes without error

## Requirement Changes

- R027: active → validated — All 5 slices complete; pipeline + orchestration spans + JSON export + OTLP + TRACEPARENT, all verified by 25+ integration tests; just ready green
- R060: active → validated — Zero eprintln! in production paths; init_tracing/TracingConfig/TracingGuard layered setup; RUST_LOG EnvFilter; 4 telemetry unit tests; all crates emit structured events
- R061: active → validated — #[instrument] on 5 pipeline functions; info_span! on 6 stage blocks; 4 pipeline_spans integration tests verify span names in subscriber output
- R062: active → validated — Orchestration root+session spans across DAG/Mesh/Gossip; cross-thread parenting in std::thread::scope; merge runner spans; 5 orchestrate_spans tests
- R063: active → validated — JsonFileLayer with atomic file writes, pruning, Vec<SpanData> format; assay traces list/show CLI; 5 core + 7 CLI tests
- R064: active → validated — telemetry feature gates all OTel deps; OTLP exporter via build_otel_layer(); graceful degradation; zero OTel deps in default build proven by cargo tree; 2 tests
- R065: active → validated — extract_traceparent + inject_traceparent in pipeline.rs; both launch paths instrumented; W3C format proven by integration test; debug log when no active span

## Forward Intelligence

### What the next milestone should know
- `init_tracing()` now has four layers: filter → fmt → json_file (if traces_dir set) → otel (if telemetry feature + endpoint). Adding a fifth layer follows the same `.with()` pattern without changing any call site.
- `TracingGuard` must be held for the program lifetime — drop flushes the non-blocking writer and shuts down the OTel provider. Post-guard-drop errors must use `eprintln!` directly.
- `assay traces list/show` reads `Vec<SpanData>` JSON (not a wrapper struct). Any change to SpanData shape is a breaking format change.
- The telemetry feature is on assay-core only (assay-cli forwards it via `assay-core/telemetry`). Any new crate needing OTel must add its own feature forwarding.
- TRACEPARENT injection requires both `telemetry` feature AND an active OTel span. Without both conditions, `inject_traceparent` is a no-op (debug log explains).

### What's fragile
- `logs_contain("span_name{")` assertions in orchestrate_spans.rs depend on tracing-test's current output format — if tracing-test changes its span formatting, all 5 tests break silently.
- `tracing_config_for()` in CLI main.rs uses a `matches!` on the parsed command — new subcommands needing traces or non-default levels require updating this function.
- The S04/S05 parallel-branch merge produced a manual merge commit. The telemetry.rs file contains both features correctly, but if a future bisect lands on a pre-merge commit, only one feature will be present.
- OTel 0.31 API surface may change in future releases — `build_otel_layer()` is the single coupling point.

### Authoritative diagnostics
- `grep -rn 'eprintln!' crates/assay-cli/src/ crates/assay-core/src/ crates/assay-tui/src/ crates/assay-mcp/src/ --include='*.rs'` — definitive eprintln check (expect 4 intentional calls only)
- `cargo test -p assay-core --test pipeline_spans` — pipeline span contract (4 tests)
- `cargo test -p assay-core --test orchestrate_spans --features orchestrate` — orchestration span contract (5 tests)
- `cargo test -p assay-core --test trace_export` — JSON file export contract (5 tests)
- `cargo test -p assay-core --test telemetry_otlp --features telemetry` — OTel + TRACEPARENT contract (2 tests)
- `cargo tree -p assay-cli | grep opentelemetry` — dep isolation check (must return empty)
- `just ready` — full workspace green

### What assumptions changed
- D127 (scoped tokio runtime) superseded by D143: assay-core already had tokio as a direct dep, making a scoped runtime unnecessary
- S04 and S05 were developed on parallel branches and required a manual merge at milestone close — the original plan assumed sequential development
- The `no-env-filter` feature on tracing-test was required for cross-crate span assertions (D136) — not anticipated in original plan
- `logs_contain()` bare substring matching caused module-path false positives, requiring the `{` suffix convention (D137) — discovered only during S03 testing

## Files Created/Modified

**S01 (Foundation):**
- `crates/assay-core/src/telemetry.rs` — new: TracingConfig, TracingGuard, init_tracing()
- `crates/assay-core/src/lib.rs` — added pub mod telemetry
- `crates/assay-core/Cargo.toml` — added tracing-subscriber, optional OTel deps, telemetry feature
- `crates/assay-cli/src/main.rs` — tracing_config_for() + init_tracing() + traces_dir + otlp_endpoint wiring
- `crates/assay-cli/src/commands/mcp.rs`, `context.rs`, `run.rs`, `gate.rs`, `harness.rs`, `worktree.rs`, `milestone.rs`, `history.rs`, `pr.rs`, `init.rs`, `plan.rs`, `spec.rs` — eprintln! → tracing
- `crates/assay-cli/Cargo.toml` — tracing.workspace, removed direct tracing-appender/subscriber
- `crates/assay-tui/src/main.rs`, `app.rs` — init_tracing() + eprintln! → tracing
- `crates/assay-tui/Cargo.toml` — tracing.workspace
- `crates/assay-core/src/history/analytics.rs`, `mod.rs` — eprintln! → tracing::warn!

**S02 (Pipeline spans):**
- `Cargo.toml` — tracing-test 0.2 with no-env-filter workspace dev-dep
- `crates/assay-core/Cargo.toml` — tracing-test dev-dep
- `crates/assay-core/src/pipeline.rs` — #[instrument] on 5 functions, info_span! on 6 stages
- `crates/assay-core/tests/pipeline_spans.rs` — new: 4 span assertion tests

**S03 (Orchestration spans):**
- `crates/assay-core/src/orchestrate/executor.rs` — DAG root + cross-thread parent + session spans
- `crates/assay-core/src/orchestrate/merge_runner.rs` — merge::run + merge::session + conflict_resolution spans
- `crates/assay-core/src/orchestrate/mesh.rs` — mesh root + routing + session spans
- `crates/assay-core/src/orchestrate/gossip.rs` — gossip root + coordinator + session spans
- `crates/assay-core/tests/orchestrate_spans.rs` — new: 5 integration tests
- `crates/assay-core/src/manifest.rs`, `pipeline.rs` — #[allow(clippy::needless_update)] on test modules

**S04 (JSON file export):**
- `crates/assay-core/src/telemetry.rs` — JsonFileLayer, SpanData, TraceFile removed (Vec<SpanData> format)
- `crates/assay-core/tests/trace_export.rs` — new: 5 integration tests
- `crates/assay-cli/src/commands/traces.rs` — new: TracesCommand with list/show
- `crates/assay-cli/src/commands/mod.rs` — traces module registration
- `justfile` — traces target added

**S05 (OTLP + TRACEPARENT):**
- `Cargo.toml` — 4 OTel workspace deps
- `crates/assay-core/Cargo.toml` — optional OTel deps + telemetry feature
- `crates/assay-cli/Cargo.toml` — telemetry feature forwarding
- `crates/assay-core/src/telemetry.rs` — build_otel_layer(), TracingGuard OTel shutdown, otlp_endpoint field
- `crates/assay-core/src/pipeline.rs` — extract_traceparent(), inject_traceparent(), TRACEPARENT injection
- `crates/assay-core/tests/telemetry_otlp.rs` — new: 2 contract tests

**Merge (S04+S05 integration):**
- `crates/assay-core/src/telemetry.rs` — combined JsonFileLayer + OTel layer in unified init_tracing()
- `crates/assay-cli/src/main.rs` — combined traces_dir + otlp_endpoint in tracing_config_for()
- `Cargo.toml` — merged OTel workspace deps
