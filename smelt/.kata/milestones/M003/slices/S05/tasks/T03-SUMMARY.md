---
id: T03
parent: S05
milestone: M003
provides:
  - monitor.rs — all 10 JobPhase variants documented with /// one-liners
  - monitor.rs — all 7 original RunState fields (job_name, phase, container_id, sessions, started_at, updated_at, pid) documented
  - monitor.rs — JobMonitor.state field documented
  - lib.rs — #![deny(missing_docs)] activated as the first inner attribute after the crate doc block
  - smelt-core — zero missing_docs errors under both default and --features forge builds
key_files:
  - crates/smelt-core/src/monitor.rs
  - crates/smelt-core/src/lib.rs
key_decisions:
  - "deny(missing_docs) inserted in lib.rs — from this point any pub item added to smelt-core without a doc comment will fail cargo build with a specific file:line compiler error"
patterns_established:
  - "JobPhase variants use active-voice one-liners describing what the phase represents (e.g. 'Container is being provisioned from the Docker image.')"
  - "RunState fields use noun-phrase one-liners anchored to their semantic role (e.g. 'Unix timestamp (seconds) when the job was started.')"
observability_surfaces:
  - "cargo build -p smelt-core 2>&1 | grep 'missing documentation' — immediate feedback on any future undocumented pub addition"
  - "RUSTDOCFLAGS='-D missing_docs' cargo doc -p smelt-core --no-deps — iterative check for incremental doc work"
duration: 5min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T03: Doc comments for monitor.rs + enable #![deny(missing_docs)]

**18 missing_docs errors closed in monitor.rs; #![deny(missing_docs)] activated in lib.rs — smelt-core now enforces complete API documentation at build time.**

## What Happened

monitor.rs had 18 undocumented public items: 10 `JobPhase` enum variants, 7 original `RunState` struct fields, and the `JobMonitor.state` field. Added `///` doc one-liners to all 18 items using active-voice phrasing for variants and noun-phrase descriptions for struct fields. The PR-related and forge-related `RunState` fields already had docs from S02/S03 work and were untouched.

After all docs were in place, added `#![deny(missing_docs)]` as the first inner attribute in `lib.rs` (immediately after the `//!` crate doc block, before `pub mod` declarations). This converts the previously advisory `RUSTDOCFLAGS="-D missing_docs"` check into a hard build-time invariant.

## Verification

```
RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep "^error" | wc -l
# → 0

RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps --features forge 2>&1 | grep "^error" | wc -l
# → 0

cargo build -p smelt-core 2>&1 | grep "^error" | head -5
# → (no output)

cargo test --workspace -q 2>&1 | grep "failed"
# → (no output) — 3 tests passed, 0 failed
```

## Diagnostics

- `cargo build -p smelt-core 2>&1 | grep "missing documentation"` — pinpoints any undocumented new `pub` items with file:line immediately
- `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps` — iterative check before triggering the full build

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/monitor.rs` — 18 new `///` doc comments on JobPhase variants, core RunState fields, JobMonitor.state
- `crates/smelt-core/src/lib.rs` — `#![deny(missing_docs)]` attribute added
