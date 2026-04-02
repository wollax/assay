---
estimated_steps: 5
estimated_files: 2
---

# T03: Doc comments for monitor.rs + enable #![deny(missing_docs)]

**Slice:** S05 — smelt-core Library API
**Milestone:** M003

## Description

Closes the final 18 missing_docs errors in `monitor.rs` and activates `#![deny(missing_docs)]` in `lib.rs`. After T02, `monitor.rs` is the only remaining file with undocumented public items. Once docs are added, the crate-level lint attribute locks the docs requirement into the build: any future `pub` item added without a doc comment will fail `cargo build`.

The 18 missing items in `monitor.rs` are:
- 10 `JobPhase` enum variants (lines 22–31): `Provisioning`, `WritingManifest`, `Executing`, `Collecting`, `TearingDown`, `Complete`, `Failed`, `Timeout`, `Cancelled`, `GatesFailed`
- 7 `RunState` struct fields (lines 37–43): `job_name`, `phase`, `container_id`, `sessions`, `started_at`, `updated_at`, `pid`
- 1 `JobMonitor.state` field (line 69)

Note: `compute_job_timeout` and the remaining `RunState` fields (`pr_url`, `pr_number`, `pr_status`, `ci_status`, `review_count`, `forge_repo`, `forge_token_env`) already have doc comments from S02/S03 — only the original fields lack them.

## Steps

1. **Read monitor.rs** — scan lines 21–70 to confirm the exact variant names and field names; confirm which items already have doc comments vs which are missing.

2. **Document JobPhase variants** — add `///` doc above each of the 10 variants (lines 22–31): `Provisioning`, `WritingManifest`, `Executing`, `Collecting`, `TearingDown`, `Complete`, `Failed`, `Timeout`, `Cancelled`, `GatesFailed`. Use active-voice one-liners describing what phase this represents. Example: `/// Container is being provisioned from the Docker image.`

3. **Document RunState fields** — add `///` docs above the 7 original fields (`job_name`, `phase`, `container_id`, `sessions`, `started_at`, `updated_at`, `pid`). Example: `/// Unique name of the job, matching the manifest `job.name` field.`

4. **Document JobMonitor.state** — add `///` doc above the `pub state: RunState` field at line 69. Example: `/// Current run state — updated at each phase transition and persisted to disk.`

5. **Add `#![deny(missing_docs)]` to lib.rs** — insert `#![deny(missing_docs)]` as the first inner attribute after the `//!` crate doc block and before any `pub mod` declarations. Then run both verification commands:
   ```bash
   RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep "^error" | wc -l
   # expected: 0

   RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps --features forge 2>&1 | grep "^error" | wc -l
   # expected: 0
   ```
   Also confirm `cargo build -p smelt-core` succeeds with the new deny attribute. Then `cargo test --workspace -q`.

## Must-Haves

- [ ] All 10 `JobPhase` variants have `///` doc comments
- [ ] All 7 original `RunState` fields (`job_name`, `phase`, `container_id`, `sessions`, `started_at`, `updated_at`, `pid`) have `///` doc comments
- [ ] `JobMonitor.state` field has a `///` doc comment
- [ ] `#![deny(missing_docs)]` is present in `lib.rs`
- [ ] `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep "^error" | wc -l` → 0
- [ ] `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps --features forge 2>&1 | grep "^error" | wc -l` → 0
- [ ] `cargo build -p smelt-core` succeeds (deny attribute does not break the build)
- [ ] `cargo test --workspace -q` passes with 0 failures

## Verification

```bash
# All missing_docs errors eliminated (no-feature and forge-feature)
RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep "^error" | wc -l
# expected: 0

RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps --features forge 2>&1 | grep "^error" | wc -l
# expected: 0

# deny attribute is in lib.rs
grep "deny(missing_docs)" crates/smelt-core/src/lib.rs
# expected: #![deny(missing_docs)]

# Build succeeds
cargo build -p smelt-core 2>&1 | grep "^error" | head -5
# expected: (no output)

# All workspace tests pass
cargo test --workspace -q 2>&1 | grep "failed"
# expected: (no output)
```

## Observability Impact

- Signals added/changed: `#![deny(missing_docs)]` — from this point, any new `pub` item in smelt-core without a doc comment will fail `cargo build`; the compiler error message includes the exact file:line of the missing doc
- How a future agent inspects this: `cargo build -p smelt-core 2>&1 | grep "missing documentation"` — immediate feedback on any undocumented new API additions
- Failure state exposed: compiler error with file:line on missing docs; `RUSTDOCFLAGS="-D missing_docs" cargo doc` for iterative checking before full deny

## Inputs

- T02 completed — 34 missing_docs errors fixed across error.rs, forge.rs, manifest.rs, git/mod.rs
- `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep "^error: missing" | wc -l` → 18 (only monitor.rs) as the entry condition

## Expected Output

- `crates/smelt-core/src/monitor.rs` — 18 new `///` doc comments on `JobPhase` variants, core `RunState` fields, `JobMonitor.state`
- `crates/smelt-core/src/lib.rs` — `#![deny(missing_docs)]` attribute added
