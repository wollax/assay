# S01: cargo doc zero-warning + deny(missing_docs) on smelt-cli — Research

**Date:** 2026-03-24
**Domain:** Rust documentation lints, rustdoc
**Confidence:** HIGH

## Summary

The slice scope is well-bounded and low-risk once the gap is quantified. There is exactly **1 cargo doc warning** (an unresolved intra-doc link in `ssh.rs:185`), which is trivially fixable. The `deny(missing_docs)` gap on smelt-cli is moderate: **~42 public items** lack doc comments across 13 source files. The items are predominantly struct fields (39 fields) and `pub mod` re-exports (10 modules), plus a few free functions and struct/enum definitions. There are no exotic items (no public traits lack docs, no public constants).

Stale `#[allow(dead_code)]` annotations: 4 total, of which 2 are in production code (1 genuinely stale, 1 debatable) and 2 are in test code (both legitimate).

The work is mechanical — add doc comments, fix one link, audit 4 annotations — with no architectural decisions or library changes required.

## Recommendation

Execute in a single pass, file by file, starting with the broken doc link fix (instant win for cargo doc zero-warning), then systematically adding doc comments to all public items in smelt-cli, and finally auditing the 4 `#[allow(dead_code)]` annotations. Enable `#![deny(missing_docs)]` on `smelt-cli/src/lib.rs` as the final step to lock the gate.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Finding missing doc items | `#![deny(missing_docs)]` lint | The compiler itself is the best tool — enable the lint and let it enumerate every missing item |
| Broken intra-doc links | `rustdoc::broken_intra_doc_links` (already on by default) | Already warns; just fix the one broken link |

## Existing Code and Patterns

- `crates/smelt-core/src/lib.rs:33` — already has `#![deny(missing_docs)]`; all public items are documented. This is the pattern to replicate in smelt-cli.
- `crates/smelt-cli/src/lib.rs` — currently 4 lines, no deny lint. Only has a module-level doc comment and two `pub mod` re-exports.
- D070 (decision) — backtick-only for `pub(crate)` types in doc links. Not directly applicable here since smelt-cli has zero `pub(crate)` items, but good to remember if any are introduced during this work.
- `crates/smelt-cli/src/main.rs` — binary crate, not subject to `missing_docs` lint (items are private to the binary). No changes needed.

## Detailed Gap Analysis

### cargo doc warning (1 total)

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `serve/ssh.rs` | 185 | `[build_ssh_args]` unresolved — both methods are on `SubprocessSshClient` impl | Change to `` [`Self::build_ssh_args`] `` or backtick-only `` `build_ssh_args` `` |

### Missing doc comments by file (42 total)

| File | Missing items | Nature |
|------|---------------|--------|
| `lib.rs` | 1 | `pub mod serve` |
| `commands/mod.rs` | 6 | All 6 `pub mod` re-exports |
| `serve/mod.rs` | 3 | 3 `pub mod` re-exports |
| `serve/config.rs` | 8 | `ServerNetworkConfig` struct + 2 fields, `ServerConfig` struct + 4 fields |
| `serve/queue.rs` | 5 | `ServerState` 4 pub fields + `new()` method |
| `serve/ssh.rs` | 7 | `SshOutput` 2 fields, `MockSshClient` builder methods (5, in `#[cfg(test)]`) |
| `serve/types.rs` | 8 | `JobId::new()`, `QueuedJob` 7 pub fields |
| `commands/*` | 0 | All Args structs and execute fns already have docs |
| `commands/status.rs` | 0 | Already documented |

**Note:** Items inside `#[cfg(test)]` modules (like `MockSshClient` builder methods in ssh.rs) are not subject to `missing_docs` at compile time because test-only code is conditionally compiled. The 5 MockSshClient methods can be documented for consistency but won't cause build failures.

Adjusted count excluding test-only items: **~37 production items** need doc comments.

### Stale `#[allow(dead_code)]` annotations (4 total)

| File | Line | Item | Verdict |
|------|------|------|---------|
| `serve/config.rs:75` | `retry_backoff_secs` field | **STALE** — field is parsed from TOML and tested but never read in production dispatch logic. The `#[allow]` comment says "used in future dispatch retry logic" — that future hasn't arrived. Keep the field (it's part of the config schema) but remove `#[allow(dead_code)]` since serde deserialization counts as a "use" for the compiler. |
| `serve/ssh.rs:532` | `MockSshClient::with_probe_result()` | **NOT STALE** — used in 15+ test callsites across `tests.rs` and `dispatch.rs`. The `#[allow(dead_code)]` is inside `#[cfg(test)]` — it may be needed if some test configurations don't call it. Verify by removing and checking if it compiles. |
| `smelt-core/src/k8s.rs:184` | `PodState` struct | **LIKELY STALE** — comment says "fields read in T03 (exec) and T04 (teardown)". The struct IS constructed (line 430) and its fields are read via `state.lock()` lookups in exec/teardown. The `#[allow]` may have been needed during incremental development but the fields are genuinely used now. Remove and verify. |
| `tests/docker_lifecycle.rs:133` | `inject_binary_to_container` fn | **NOT STALE** — test helper function, used at line 1340. The `#[allow]` is needed because the function is only called in one `#[ignore]` test that requires `ASSAY_SOURCE_DIR`. Without the allow, the compiler warns when that test is not compiled. Keep as-is — it's a test file, not production code. |

## Constraints

- `#![deny(missing_docs)]` must be added to `crates/smelt-cli/src/lib.rs` — this only covers the library crate, not `main.rs` (binary items are private and exempt).
- Doc comments must be added BEFORE enabling the deny lint, or the build breaks immediately.
- `cargo doc --workspace --no-deps` must be run after all changes to confirm zero warnings.
- `cargo test --workspace` must pass after all changes — doc comments don't affect behavior but `#[allow]` removal could surface compiler warnings-as-errors.
- The `cargo clean --doc` workaround (for the `search.index/path/` filesystem error) is needed if the doc cache is corrupted. This is a local build artifact issue, not a code issue.

## Common Pitfalls

- **Intra-doc link resolution scope** — `[`FnName`]` in a doc comment resolves relative to the current module, not the impl block. For methods on the same impl, use `[`Self::method_name`]` or backtick-only. The broken link on line 185 of ssh.rs is exactly this pitfall.
- **`missing_docs` doesn't check `#[cfg(test)]` items** — MockSshClient methods inside `#[cfg(test)]` won't trigger the lint. Don't waste time documenting test-only code for lint compliance (though consistency is nice).
- **`missing_docs` checks struct fields** — Every `pub` field on a `pub` struct needs a `///` comment, not just the struct itself. This is the bulk of the work (39 of 42 items are fields or module re-exports).
- **Removing `#[allow(dead_code)]` on serde fields** — Fields that are deserialized by serde but never read in Rust code may or may not trigger `dead_code` warnings depending on whether the struct itself is constructed/used. Test by removing the annotation and compiling.

## Open Risks

- **`retry_backoff_secs` dead_code removal** — Removing the `#[allow(dead_code)]` may or may not trigger a warning. If it does, the field genuinely isn't used in production code and should either get a `// TODO: wire into dispatch retry` comment or be removed from the config schema. Low risk — either outcome is fine.
- **`PodState` dead_code removal** — Same pattern. The struct and fields appear to be genuinely used now (constructed at line 430, read via Mutex lookups). Removing the allow should be clean but needs a compile check.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust / rustdoc | N/A | No specialized skill needed — this is standard Rust lint/doc work |

## Sources

- Codebase inspection (grep/read of all smelt-cli source files)
- Rust reference: `missing_docs` lint behavior (known from training data — stable Rust behavior, no lookup needed)
- Decision register: D070 (backtick-only for pub(crate) types)
