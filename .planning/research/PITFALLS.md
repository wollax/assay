# Pitfalls Research: v0.2.0 Features

**Date:** 2026-03-02
**Scope:** Common mistakes when adding run history persistence, required/advisory enforcement levels, agent gate evaluation recording, and foundation hardening to the existing Assay v0.1.0 codebase.

**Supersedes:** v0.1.0 PITFALLS.md (all v0.1.0 pitfalls P-01 through P-20 have been mitigated in the shipped codebase).

---

## Critical Pitfalls

### P-21: `deny_unknown_fields` blocks schema evolution on user-facing types

**Area:** Schema Evolution (assay-types)
**Confidence:** High (verified in codebase)

**What goes wrong:** Assay v0.1.0 uses `#[serde(deny_unknown_fields)]` on `Spec`, `Criterion`, `Config`, and `GatesConfig`. When v0.2 adds new fields (e.g., `enforcement` on `Criterion`, `results_dir` on `Config`), any spec or config file written by v0.2 that includes the new fields becomes **unparseable by v0.1 Assay**. This is backward-compatible (old files work with new code) but not forward-compatible (new files break old code).

More critically: if v0.2 adds a field with `#[serde(default)]` so it is optional, a user who writes a v0.2 spec file and later downgrades will get a hard deserialization error, not a warning. The `deny_unknown_fields` attribute is all-or-nothing; there is no whitelist mechanism in serde (serde issue #1864 confirms this).

**Warning signs:**
- Tests pass but users report "unknown field" errors after downgrading
- CI/CD using different Assay versions across environments
- Spec files shared between projects using different Assay versions

**Prevention:**
- Add new fields with `#[serde(default, skip_serializing_if = "...")]` so they are omitted from serialized output when unset, preserving roundtrip compatibility
- Document the minimum Assay version required for each field in spec/config schemas
- Consider a `version` or `schema_version` field on `Config` (not on Spec, to avoid per-file overhead) that gates field parsing
- Write explicit backward-compatibility tests: parse a v0.1.0-era spec file with v0.2 code and verify no regressions
- **Do not remove `deny_unknown_fields`** -- it catches real typos. Instead, evolve fields carefully

**Which phase should address it:** The first phase that adds any new field to `Spec`, `Criterion`, or `Config`.

---

### P-22: Concurrent result file writes corrupt run history

**Area:** Run History Persistence (file I/O)
**Confidence:** High (architectural analysis)

**What goes wrong:** Run history persists `GateRunSummary` results to `.assay/results/`. If two gate evaluations run concurrently (e.g., CLI `assay gate run --all` in parallel, MCP server handling simultaneous `gate_run` calls, or CI running alongside a developer), they may:
1. Write to the same result file simultaneously, producing corrupted JSON/TOML
2. Read a partially-written file when querying history
3. Race on directory creation (`std::fs::create_dir_all` is not atomic)

The v0.1.0 `.assay/.gitignore` already ignores `results/`, but the directory does not exist yet -- it must be created on first write.

**Warning signs:**
- Truncated or malformed JSON in result files
- Intermittent "file not found" or "invalid JSON" errors in CI
- Result files containing interleaved content from two runs

**Prevention:**
- Use atomic file writes: write to a temporary file in the same directory, then `rename()` to the final path. The `tempfile` crate (already in workspace dependencies) supports this via `NamedTempFile::persist()`
- Use unique filenames per run (e.g., `{spec_name}-{timestamp}-{pid}.json`) rather than overwriting a single file per spec
- Call `fsync` / `File::sync_all()` before `persist()` if crash safety matters (result files are ephemeral, so this may be overkill)
- For reading history, use a listing + parse approach (glob `results/*.json`) rather than maintaining an index file that requires locking
- Avoid advisory file locking (`fs2`, `fd-lock`) for result files; the complexity is not justified when unique filenames solve the problem

**Which phase should address it:** The run history persistence phase.

---

### P-23: Unbounded result file accumulation exhausts disk

**Area:** Run History Persistence (disk space)
**Confidence:** Medium (depends on usage patterns)

**What goes wrong:** Each `gate_run` invocation writes a result file. In CI environments or agent loops that repeatedly run gates, the `.assay/results/` directory grows unboundedly. A gate with 10 criteria, each producing 64KB of stdout/stderr evidence, generates ~640KB per run. At 100 runs/day, that is 64MB/day -- manageable for a human, but an agent in a tight retry loop could run gates thousands of times.

**Warning signs:**
- `.assay/results/` directory consuming gigabytes
- `du -sh .assay/` surprising users
- Disk full errors in CI

**Prevention:**
- Implement a retention policy: keep only the last N result files per spec (default: 10-20)
- Truncate stdout/stderr in persisted results the same way the in-memory `GateResult` does (already has `MAX_OUTPUT_BYTES = 65_536`)
- Add a `assay results prune` CLI command for manual cleanup
- Do not store evidence in result files by default; store summaries only, with `--include-evidence` writing the full data
- Prune on write: when persisting a new result, delete oldest files beyond the retention limit

**Which phase should address it:** The run history persistence phase, as a built-in constraint rather than an afterthought.

---

### P-24: Agent self-grading bias in agent-evaluated gates

**Area:** Agent Evaluation Trust
**Confidence:** High (well-documented in AI evaluation literature)

**What goes wrong:** When the same AI agent that implemented code also evaluates whether it meets criteria (via a `prompt` field on `Criterion`), the agent has a systematic bias toward passing its own work. Research shows this manifests as:
1. **Anchoring bias**: The agent anchors on its own implementation decisions and evaluates criteria through that lens
2. **Sycophantic evaluation**: The agent avoids failing its own work to appear competent
3. **Criterion reinterpretation**: The agent interprets ambiguous criteria favorably when evaluating its own output
4. **Missing edge cases**: The agent doesn't test failure modes it didn't consider during implementation

Google Cloud's 2025 agent trust research emphasizes that "without a clear audit trail of why the AI made its decisions, it is nearly impossible to diagnose failures or prove due diligence."

**Warning signs:**
- Agent-evaluated gates pass at suspiciously higher rates than command gates
- Agent evaluation justifications are vague or tautological ("The code meets the criterion because it implements the required functionality")
- Agent passes criteria that would fail deterministic checks

**Prevention:**
- **Separate evaluator from implementer**: Run agent evaluations in a fresh agent context without the conversation history of the implementing agent
- Record the full evaluation reasoning in `GateResult` (not just pass/fail) so humans can audit
- Require deterministic gates alongside agent gates: every spec should have at least one command-based criterion as an anchor
- Add a `confidence` field to agent `GateResult` so low-confidence passes can be flagged
- Default agent gates to `enforcement: "advisory"` (not `required`) until trust calibration shows they are reliable
- Never allow an agent gate result to override a failed command gate result

**Which phase should address it:** The agent gate evaluation phase, with the enforcement level phase establishing the advisory/required distinction first.

---

## Moderate Pitfalls

### P-25: Enforcement level changes silently weaken existing gate results

**Area:** Required/Advisory Gate Enforcement
**Confidence:** Medium (design analysis)

**What goes wrong:** When adding `enforcement: "required"` vs `"advisory"` to criteria/gates, the enforcement level determines whether a gate failure blocks progression. The pitfall is twofold:
1. **Default assignment**: If existing criteria default to `required`, existing users see no behavior change. If they default to `advisory`, existing gate failures that previously blocked the CLI exit code 1 now become non-blocking, silently weakening quality.
2. **MCP response ambiguity**: The MCP `gate_run` response currently returns aggregate `passed`/`failed`/`skipped` counts. Adding enforcement levels means `failed` could mean "failed-required" (blocking) or "failed-advisory" (informational). Agents must distinguish these to know whether to retry.

**Warning signs:**
- Agents stop retrying gate failures that used to block
- CLI exit code changes meaning (was: any failure = exit 1, now: only required failures = exit 1)
- Users confused by "failures" that don't block

**Prevention:**
- Default `enforcement` to `"required"` for backward compatibility -- all existing behavior is preserved
- Extend the `gate_run` MCP response with separate counts: `required_passed`, `required_failed`, `advisory_passed`, `advisory_failed`
- CLI exit code policy: exit 1 if any `required` criterion fails; advisory failures produce warnings but exit 0
- Document the enforcement level clearly in spec examples and schema descriptions
- Add an `enforcement` field to `CriterionSummary` in the MCP response so agents see enforcement per-criterion, not just in aggregate

**Which phase should address it:** The enforcement level phase, before agent evaluation is added (agent gates should be `advisory` by default).

---

### P-26: `GateRunSummary` is not serializable for persistence

**Area:** Run History Persistence (type design)
**Confidence:** High (verified in codebase)

**What goes wrong:** `GateRunSummary` and `CriterionResult` in `assay-core::gate` derive only `Serialize` (via `#[derive(Debug, Clone, Serialize)]`). They do **not** derive `Deserialize` or `JsonSchema`. To persist and reload run history, these types need full serde roundtrip support. However, they live in `assay-core` (not `assay-types`), and `assay-core` does not export them as public DTOs intended for persistence.

Additionally, `CriterionResult.result` is `Option<GateResult>` where `GateResult` is from `assay-types` and already has full serde derives. The mismatch is specifically on the wrapper types in `assay-core`.

**Warning signs:**
- Cannot deserialize result files back into structured types
- Must define parallel types for persistence, creating duplication
- Tests cannot assert on deserialized run history

**Prevention:**
- Add `Deserialize` and `JsonSchema` to `GateRunSummary` and `CriterionResult` in `assay-core`
- Alternatively, define a `RunRecord` type in `assay-types` that is the persistence DTO, distinct from the computed `GateRunSummary`
- If using a separate persistence type, implement `From<GateRunSummary> for RunRecord` as a one-way mapping (core -> types)
- Add a `run_id` field (UUID or timestamp-based) to the persistence type for indexing
- Include `assay_version` in the persisted record for future schema migration

**Which phase should address it:** The run history persistence phase, as a prerequisite to writing result files.

---

### P-27: New MCP tools break existing plugin skills

**Area:** MCP Tool Addition (backward compatibility)
**Confidence:** Medium (integration analysis)

**What goes wrong:** v0.2 adds new MCP tools (e.g., `gate_history`, `gate_record_agent_eval`). The Claude Code plugin's `SKILL.md` files reference specific tool names and response shapes. If existing tool response shapes change (e.g., `gate_run` response gains `enforcement` fields), skills that parse the response may break. More subtly, the MCP `tools/list` response grows, consuming more of the agent's context window.

The existing plugin at `plugins/claude-code/` has skills that assume the current `gate_run` response format: `{spec_name, passed, failed, skipped, total_duration_ms, criteria: [{name, status, exit_code, ...}]}`.

**Warning signs:**
- Agent reports "unexpected field" or ignores new fields in gate_run responses
- Agent's context window fills up from tool listing, reducing available context for actual work
- Plugin skills give incorrect advice based on stale response format assumptions

**Prevention:**
- Make all response schema changes additive (new optional fields, never remove or rename existing fields)
- Add `enforcement` as an optional field on `CriterionSummary` with `#[serde(skip_serializing_if = "Option::is_none")]` so existing responses are unchanged
- Update `SKILL.md` files to handle new response fields, but make the handling graceful (new fields are informational, not required for the skill to function)
- Keep MCP tool count low (the v0.1.0 server has 3 tools; aim for 5-6 max in v0.2)
- Test plugin skills end-to-end with the updated MCP server before release

**Which phase should address it:** Every phase that modifies MCP tool schemas or adds new tools.

---

### P-28: Result file path length exceeds OS limits on deep project paths

**Area:** Run History Persistence (file I/O)
**Confidence:** Low (edge case, but painful when hit)

**What goes wrong:** Result file paths follow the pattern `.assay/results/{spec_name}-{timestamp}-{pid}.json`. On Windows, `MAX_PATH` is 260 characters. A deep project path (`C:\Users\user\Documents\Projects\client\workspace\project\`) plus a long spec name plus a full ISO 8601 timestamp can exceed this limit. On macOS/Linux, `NAME_MAX` is 255 bytes for a single filename component, which a long spec name could approach.

**Warning signs:**
- "File name too long" errors on result file creation
- Tests pass in shallow temp directories but fail in deep project paths

**Prevention:**
- Truncate spec names in filenames (first 50 chars) and use a hash suffix for uniqueness
- Use compact timestamps (e.g., `20260302T143022` not `2026-03-02T14:30:22.123456+00:00`)
- Validate total path length before writing and return a clear error
- Alternatively, use a flat numeric ID scheme: `results/00001.json`, `results/00002.json`

**Which phase should address it:** The run history persistence phase (filename format decision).

---

### P-29: Hardening refactors break the `#[non_exhaustive]` error type contract

**Area:** Hardening (error types)
**Confidence:** Medium (design analysis)

**What goes wrong:** `AssayError` is `#[non_exhaustive]`, which means adding variants is non-breaking. However, hardening often involves **reorganizing** error variants (merging similar ones, renaming for clarity, changing field types). Any variant that the MCP server pattern-matches on in `domain_error()` or that the CLI matches in `main.rs` will break silently if renamed. The MCP server currently converts all `AssayError` variants to a text string via `err.to_string()`, which is resilient, but the CLI matches specific variants (e.g., `AssayError::Io { source, .. } if source.kind() == std::io::ErrorKind::NotFound`).

**Warning signs:**
- CLI error messages change unexpectedly after refactoring
- Match arms in CLI become dead code after variant reorganization
- `cargo clippy` warns about unreachable patterns

**Prevention:**
- Before reorganizing error variants, grep all match sites: `rg 'AssayError::' crates/`
- Add exhaustive test coverage for error display strings (insta snapshots) before refactoring
- When renaming variants, use `#[deprecated]` type aliases or add both old and new variants temporarily
- Keep the MCP server's `domain_error()` converting via `Display` (`.to_string()`) rather than matching on specific variants
- Run `just ready` after every error type change to catch breakage immediately

**Which phase should address it:** The hardening phase, with match-site auditing as a prerequisite step.

---

### P-30: Agent evaluation recording leaks sensitive command output

**Area:** Agent Evaluation Recording (security)
**Confidence:** Medium (depends on what gates run)

**What goes wrong:** When recording agent gate evaluations, the `GateResult` captures stdout/stderr from the agent's evaluation process. If the agent subprocess has access to environment variables, API keys, or file contents, these may appear in stdout/stderr and get persisted to `.assay/results/`. While `.assay/results/` is gitignored, it exists on disk and may be accessible to other tools, agents, or users on the same machine.

The v0.1.0 codebase already captures stdout/stderr from command gates (up to 64KB). The same capture mechanism applied to agent evaluation subprocesses has a larger attack surface because agent processes typically have broader system access than simple shell commands.

**Warning signs:**
- API keys or tokens appearing in result files
- Result files containing file contents from outside the project
- Agent evaluation stderr including debug output with credentials

**Prevention:**
- Sanitize captured output before persistence: strip common secret patterns (`Bearer `, `sk-`, `ghp_`, etc.)
- Apply the existing `MAX_OUTPUT_BYTES` truncation to agent evaluation output (already implemented for command gates)
- Document that result files may contain sensitive information and should not be committed to version control (the `.gitignore` pattern already covers this)
- Consider a `--redact` flag on result persistence that masks potential secrets
- For agent evaluation specifically: use `Command::env_clear()` and whitelist only necessary environment variables (mirrors v0.1.0 pitfall P-17's guidance, which was documented but not implemented)

**Which phase should address it:** The agent evaluation recording phase.

---

## Minor Pitfalls

### P-31: Test coverage regressions during hardening refactors

**Area:** Hardening (test infrastructure)
**Confidence:** Medium (process risk)

**What goes wrong:** v0.1.0 ships with 119 tests across the workspace. Hardening phases that refactor internal APIs (e.g., extracting common patterns, reorganizing modules) risk breaking existing tests. The temptation is to delete or weaken tests that no longer compile rather than updating them to test the refactored code. This is especially risky with insta snapshot tests, where `cargo insta review` can silently accept wrong snapshots.

**Warning signs:**
- Test count decreasing after a refactoring PR
- `cargo insta review --accept` run without manual inspection
- Tests that assert on implementation details (specific function signatures) rather than behavior

**Prevention:**
- Record test count before and after each hardening change: `cargo test 2>&1 | tail -1`
- Never decrease test count during hardening without documented justification
- Use `just ready` (which includes `fmt-check + lint + test + deny`) as the gate for every change
- For insta snapshots: review diffs manually, do not auto-accept
- Add a CI check that fails if test count drops (can be a simple `wc -l` on test output)

**Which phase should address it:** The hardening phase, as a continuous process check.

---

### P-32: `GateKind` enum extension requires matching in multiple crates

**Area:** Schema Evolution (type design)
**Confidence:** High (verified in codebase)

**What goes wrong:** Adding a new `GateKind` variant (e.g., `AgentEval { prompt: String }`) requires updates in:
1. `assay-types/src/gate.rs` -- the enum definition
2. `assay-core/src/gate/mod.rs` -- the `evaluate()` match arm
3. `assay-mcp/src/server.rs` -- the `format_gate_response()` mapping
4. `assay-cli/src/main.rs` -- potentially the display logic
5. Test files across all of the above

Because `GateKind` uses `#[serde(tag = "kind")]` (internally tagged), the new variant also needs TOML roundtrip tests to confirm the tag value. If the variant has different fields than existing ones, the schemars-generated JSON Schema changes, which must be verified against MCP client expectations.

**Warning signs:**
- "non-exhaustive patterns" compiler error in downstream crates after adding a variant
- New variant works in tests but produces wrong JSON Schema
- TOML serialization of new variant has unexpected format

**Prevention:**
- Add the variant to `GateKind` with a TOML roundtrip test in the same PR
- Grep all match sites before opening the PR: `rg 'GateKind::' crates/`
- Generate and diff JSON schemas before and after the change
- The `evaluate()` function in `assay-core` should handle the new variant even if it is a stub (`todo!()` is acceptable during development but must be replaced before merge)
- Consider a `GateKind::Unknown` catch-all variant for forward compatibility (but this conflicts with `deny_unknown_fields` on the enum's internal tag)

**Which phase should address it:** The agent gate evaluation phase (when `AgentEval` variant is added).

---

### P-33: Run history query performance degrades with file-per-run storage

**Area:** Run History Persistence (performance)
**Confidence:** Low (unlikely to matter at v0.2 scale)

**What goes wrong:** Querying run history (e.g., "show last 5 runs for spec X") requires listing the directory, parsing filenames for timestamps, sorting, and deserializing the top N files. With thousands of result files, directory listing becomes slow on some filesystems (especially network-mounted or Windows NTFS with many small files).

**Warning signs:**
- `assay results show` command taking seconds
- MCP `gate_history` tool timing out
- File system inode limits reached on ext4

**Prevention:**
- Implement retention (P-23) to bound the number of files per spec
- Use a compact index file (`.assay/results/index.jsonl`, one line per run with metadata) for fast queries, writing full results to individual files
- If index file corruption is a concern, rebuild it from individual result files on demand
- For v0.2, file-per-run is fine; consider SQLite only if v0.3+ needs complex queries

**Which phase should address it:** The run history persistence phase (design decision, not implementation blocker).

---

### P-34: `spawn_blocking` bridge does not propagate panics clearly for new gate types

**Area:** MCP Server (async/sync boundary)
**Confidence:** Medium (extends v0.1.0 P-03)

**What goes wrong:** The MCP server wraps `evaluate_all()` in `tokio::task::spawn_blocking()`. When adding agent evaluation (which may invoke a subprocess that itself uses an LLM API), the blocking task duration increases significantly. If the agent evaluation process hangs or panics inside `spawn_blocking`, the MCP server's error handling maps the `JoinError` to a generic `"gate evaluation panicked"` message. With multiple gate types (command, file, agent), the panic source becomes ambiguous.

**Warning signs:**
- MCP server returns "gate evaluation panicked" with no context on which criterion or gate type caused it
- Agent evaluation hangs indefinitely because the blocking task has no independent timeout
- Tokio's blocking thread pool exhaustion when multiple agent evaluations run concurrently

**Prevention:**
- Add per-gate-type context to the `JoinError` mapping: include the criterion name and gate kind in the error message
- Apply a per-criterion timeout inside the blocking task (already done for command gates; extend to agent gates)
- Configure tokio's blocking thread pool size if agent evaluations are expected to be concurrent (`tokio::runtime::Builder::max_blocking_threads()`)
- Consider using `tokio::task::spawn_blocking` per-criterion rather than per-spec to isolate panics

**Which phase should address it:** The agent evaluation recording phase.

---

## Integration Pitfalls (Cross-Cutting)

### P-35: Run history persistence and MCP response format must agree on result shape

**Area:** Run History + MCP Server
**Confidence:** High (architectural coupling)

**What goes wrong:** The MCP `gate_run` tool returns a `GateRunResponse` (defined in `assay-mcp/src/server.rs`) that is a projection of `GateRunSummary` (defined in `assay-core/src/gate/mod.rs`). Run history persistence writes `GateRunSummary` (or a derived `RunRecord`) to disk. If the persistence format diverges from the MCP response format, loading a historical result and serving it via a `gate_history` MCP tool requires a format conversion layer. Worse, if the persistence format drops fields that the MCP response includes (or vice versa), information is lost.

**Warning signs:**
- Historical results served via MCP are missing fields that live results have
- Two separate serialization formats for the same conceptual data
- Maintenance burden of keeping three types in sync (`GateRunSummary`, `RunRecord`, `GateRunResponse`)

**Prevention:**
- Define a single canonical result type in `assay-types` (e.g., `RunRecord`) that is used for both persistence and MCP responses
- The MCP server projects from `RunRecord` to `GateRunResponse` (a subset), not from `GateRunSummary`
- Keep the projection one-directional: `GateRunSummary -> RunRecord` on persist, `RunRecord -> GateRunResponse` on serve
- Test that a serialized-then-deserialized `RunRecord` produces the same `GateRunResponse` as a fresh evaluation

**Which phase should address it:** The run history persistence phase (design) and the MCP tool addition phase (consumption).

---

### P-36: Enforcement levels interact with agent evaluation trust in non-obvious ways

**Area:** Enforcement Levels + Agent Evaluation
**Confidence:** Medium (design coupling)

**What goes wrong:** The enforcement level (`required` vs `advisory`) and the gate type (`Command` vs `AgentEval`) create a 2x2 matrix of behaviors:

| | Required | Advisory |
|---|---|---|
| **Command gate** | Fails -> blocks, exit 1 | Fails -> warning, exit 0 |
| **Agent gate** | Fails -> blocks, exit 1 | Fails -> warning, exit 0 |

The pitfall: a `required` + `AgentEval` gate has the same blocking power as a `required` + `Command` gate, but the trust level is fundamentally different. A command gate produces deterministic, reproducible results. An agent gate's pass/fail is probabilistic and subject to the biases in P-24. Giving both the same blocking power means an unreliable agent evaluation can block production workflows.

**Warning signs:**
- Users set agent gates to `required` because they want quality enforcement, not realizing the evaluation is non-deterministic
- Agent gate results fluctuate between runs (pass one time, fail the next) creating flaky "quality" gates
- Teams lose trust in the gate system because required gates fail spuriously

**Prevention:**
- Default agent gates to `enforcement: "advisory"` regardless of what the user specifies, with a CLI warning: "agent-evaluated gates are advisory by default; use `--force-required` to override"
- Add a `trust_level` or `determinism` metadata field to `GateResult` so downstream consumers (CLI, MCP, TUI) can display appropriate caveats
- In the CLI, display agent gate results with a visual distinction (e.g., a `~` prefix instead of a checkmark)
- In the MCP response, include `deterministic: false` on agent gate results so agents can weigh them appropriately
- Consider a calibration mechanism: track agent gate agreement with command gates over time, and promote to `required` only when agreement exceeds a threshold

**Which phase should address it:** The enforcement level phase should establish the framework; the agent evaluation phase should implement the trust constraints.

---

### P-37: Hardening changes to `assay-core` public API break both CLI and MCP consumers

**Area:** Hardening (API stability)
**Confidence:** High (workspace coupling)

**What goes wrong:** `assay-core` exports public functions (`gate::evaluate`, `gate::evaluate_all`, `spec::load`, `spec::scan`, `config::load`, `init::init`) and types (`GateRunSummary`, `CriterionResult`, `AssayError`) consumed by both `assay-cli` and `assay-mcp`. Hardening changes that modify function signatures (e.g., adding a parameter, changing return types, reorganizing modules) break both consumers simultaneously. Because the workspace compiles all crates together, this surfaces as compile errors, not runtime failures -- but it means every core API change requires coordinated updates across at minimum 2 downstream crates.

**Warning signs:**
- A "quick refactor" in `assay-core` cascades into changes in 3+ files across crates
- Function signature changes require updating both CLI and MCP server in the same PR
- PR size bloat from cross-crate cascades

**Prevention:**
- Add changes behind new functions/methods rather than modifying existing signatures. Deprecate old ones with `#[deprecated]` for one release cycle
- Use builder patterns or option structs for functions likely to gain parameters (e.g., `EvaluateOptions { cli_timeout, config_timeout, enforcement_filter }` instead of adding positional parameters)
- Extract common patterns into internal helper functions within `assay-core` rather than changing the public API surface
- Run `just ready` after every change to catch all downstream breakage immediately
- Consider marking `assay-core`'s public API with `/// # Stability` doc comments indicating which functions are stable vs. experimental

**Which phase should address it:** The hardening phase (API surface review), with continuous attention during all phases.

---

## Summary

| ID | Severity | Area | Core Issue |
|---|---|---|---|
| P-21 | Critical | Schema evolution | `deny_unknown_fields` blocks forward compatibility |
| P-22 | Critical | Run history I/O | Concurrent writes corrupt result files |
| P-23 | Critical | Run history I/O | Unbounded result file accumulation |
| P-24 | Critical | Agent evaluation | Self-grading bias in agent-evaluated gates |
| P-25 | Moderate | Enforcement levels | Default level choice silently changes behavior |
| P-26 | Moderate | Run history types | `GateRunSummary` lacks `Deserialize` for persistence |
| P-27 | Moderate | MCP tools | New tools/fields break existing plugin skills |
| P-28 | Moderate | Run history I/O | Result file path length exceeds OS limits |
| P-29 | Moderate | Hardening | Error type refactors break match sites |
| P-30 | Moderate | Agent evaluation | Recorded evaluation output leaks secrets |
| P-31 | Minor | Hardening | Test coverage regression during refactors |
| P-32 | Minor | Schema evolution | `GateKind` extension requires multi-crate updates |
| P-33 | Minor | Run history | File-per-run query performance at scale |
| P-34 | Minor | MCP server | `spawn_blocking` error context for new gate types |
| P-35 | Integration | History + MCP | Result shape divergence between persistence and API |
| P-36 | Integration | Enforcement + Agent | Trust mismatch in required agent gates |
| P-37 | Integration | Hardening + API | Core API changes cascade across consumers |

---

## Phase Allocation

| Phase | Pitfalls to Address |
|---|---|
| **Run History Persistence** | P-22, P-23, P-26, P-28, P-33, P-35 |
| **Enforcement Levels** | P-21, P-25, P-36 |
| **Agent Gate Evaluation** | P-24, P-30, P-32, P-34, P-36 |
| **MCP Tool Addition** | P-27, P-35 |
| **Hardening** | P-29, P-31, P-37 |
| **All Phases** | P-21 (any field addition), P-37 (any API change) |

---

## Sources

- [serde `deny_unknown_fields` whitelist limitation (Issue #1864)](https://github.com/serde-rs/serde/issues/1864)
- [serde `deny_unknown_fields` and `skip` incompatibility (Issue #2121)](https://github.com/serde-rs/serde/issues/2121)
- [Atomic file writes in Rust (rust-lang forum)](https://users.rust-lang.org/t/how-to-write-replace-files-atomically/42821)
- [tempfile `NamedTempFile::persist` docs](https://docs.rs/tempfile/latest/tempfile/struct.NamedTempFile.html)
- [atomic-write-file crate](https://docs.rs/atomic-write-file)
- [fs4 (fork of fs2) cross-platform file locking](https://lib.rs/crates/fs4)
- [PSA: Avoid Data Corruption by Syncing to the Disk](https://blog.elijahlopez.ca/posts/data-corruption-atomic-writing/)
- [MCP Tools Specification](https://modelcontextprotocol.io/specification/draft/server/tools)
- [MCP Tool Schema Guide (Merge)](https://www.merge.dev/blog/mcp-tool-schema)
- [Google Cloud: Lessons from 2025 on agents and trust](https://cloud.google.com/transform/ai-grew-up-and-got-a-job-lessons-from-2025-on-agents-and-trust)
- [AI Agent Evaluation: Comprehensive Framework (LXT)](https://www.lxt.ai/blog/ai-agent-evaluation/)
- [Evaluations for the Agentic World (McKinsey, 2026)](https://medium.com/quantumblack/evaluations-for-the-agentic-world-c3c150f0dd5a)
- [The AI Test Agent's Dilemma: Ethics of Autonomous QA (2025)](https://www.askui.com/blog-posts/ai-qa-ethics-dilemma-2025)
- [Measurement Imbalance in Agentic AI Evaluation](https://arxiv.org/html/2506.02064v2)

---

*Research completed: 2026-03-02*
