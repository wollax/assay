# Triage Summary

**Date:** 2026-03-07
**Total issues reviewed:** 143
**Previously closed:** 19
**Closed in this triage:** 17
**Remaining open:** 107

## Closed Issues (17 newly closed)

All closures verified against current source code.

| Issue | Resolution |
|-------|-----------|
| cli-error-propagation | `main()` uses async with `run() -> Result<i32>` |
| core-error-types | `AssayError` fully established with 20+ variants |
| deny-multiple-versions | `deny.toml` uses `multiple-versions = "deny"` (Phase 19) |
| deny-source-controls | `deny.toml` uses `unknown-registry = "deny"` (Phase 19) |
| dogfood-checkpoint | `self-check.toml` exists in `.assay/specs/` (Phase 19) |
| phase3-serde-hygiene | 89 `skip_serializing_if` annotations across 8 type files |
| phase3-truncation-metadata | `truncated` and `original_bytes` fields exist on `GateResult` |
| test-coverage-gaps-phase3 | Deserialization failure tests + JSON roundtrip tests added |
| assay-dir-string-literal | `ASSAY_DIR_NAME` constant extracted and used |
| bare-invocation-exit-code | Returns `Ok(1)` for bare invocation outside project |
| mcp-gate-run-timeout-param | `timeout: Option<u64>` on `GateRunParams` |
| mcp-response-docs | All response structs have field-level doc comments |
| mcp-spec-list-silent-errors | `SpecListResponse` includes `errors` field |
| mcp-spec-not-found-unused | `SpecNotFound` now constructed in `load_spec_entry()` |
| mcp-tool-description-accuracy | Descriptions mention optional fields and skipped criteria |
| mcp-tool-handler-test-coverage | Integration tests in `crates/assay-mcp/tests/` |
| mcp-working-dir-validation | `is_dir()` check before gate evaluation |
| cli-advisory-failure-display | Advisory failures display as WARN with separate `warned` counter |
| mcp-response-struct-docs | `GateRunResponse` and `CriterionSummary` have doc comments |

## Priority Tiers

### Must-Fix (target: v0.2.1)

Correctness issues, API design problems, and missing validation.

#### Types

| Issue | Area | Summary |
|-------|------|---------|
| gaterunrecord-deny-unknown-fields | types | `deny_unknown_fields` on output type breaks forward-compatibility |
| enforcement-summary-serde-default | types | `EnforcementSummary` fields lack `#[serde(default)]` for schema compat |
| enforcement-summary-field-docs | types | Public fields missing doc comments, inconsistent with crate convention |
| gate-run-summary-field-ambiguity | types | `passed`/`failed` fields ambiguous with enforcement levels |
| gate-run-summary-serialize | types | `skip_serializing_if` on `results` breaks API schema consistency |
| criterion-structural-duplication | types | `GateCriterion` and `Criterion` near-identical, maintenance burden |
| gates-spec-criteria-serde-default | types | Missing `#[serde(default)]` on `GatesSpec.criteria` |
| gaterunrecord-assay-version-serde-default | types | Missing `#[serde(default)]` on `assay_version` for legacy resilience |

#### Evaluation

| Issue | Area | Summary |
|-------|------|---------|
| evaluate-gates-duplication | evaluation | `evaluate_all` and `evaluate_all_gates` contain duplicated logic |
| spec-validation-enforcement-duplication | evaluation | Enforcement validation block duplicated in `validate()`/`validate_gates_spec()` |
| spec-parse-errors-silent | evaluation | Parse errors silently swallowed in `spec_get` and `spec show` |

#### MCP

| Issue | Area | Summary |
|-------|------|---------|
| mcp-failure-reason-stdout-fallback | MCP | Failure reason only checks stderr, misses stdout-only errors |
| gate-finalize-untyped-response | MCP | `gate_finalize` uses untyped JSON while other tools use typed structs |
| gate-history-entry-missing-passed-counts | MCP | `GateHistoryEntry` missing `required_passed`/`advisory_passed` fields |
| gate-history-silent-entry-skip | MCP | Unreadable history entries silently dropped with no agent signal |
| gate-history-unused-config-load | MCP | `gate_history` loads full config unnecessarily |
| session-timeout-dead-wd-capture | MCP | `wd_string` captured in timeout task but explicitly suppressed |
| spec-get-silent-feature-spec-error | MCP | Feature spec load errors silently swallowed with `.ok()` |

#### CLI

| Issue | Area | Summary |
|-------|------|---------|
| cli-enforcement-check-duplication | CLI | Enforcement check block duplicated in `handle_gate_run_all` and `handle_gate_run` |
| spec-new-no-rollback | CLI | Multi-file spec creation has no rollback on partial failure |

#### Guard

| Issue | Area | Summary |
|-------|------|---------|
| guard-checkpoint-current-dir | guard | `try_save_checkpoint` uses `current_dir()` instead of stored project dir |
| guard-threshold-prune-errors-swallowed | guard | Failed prunes return `Ok(())` but consume circuit breaker budget |
| guard-context-pct-duplicated | guard | Context percentage calculation duplicated with divergent error handling |
| guard-watcher-unnecessary-arc-mutex | guard | `Arc<Mutex<OsString>>` for immutable `target_name` in hot path |
| guard-daemon-run-unix-only | guard | No compile error or runtime error on non-Unix platforms |
| guard-pid-no-fsync | guard | `create_pid_file` doesn't fsync, crash could leave partial PID |
| guard-cli-session-glob-ambiguity | guard | Auto-discovery could silently pick wrong session with multiple candidates |
| guard-circuit-breaker-no-reset | guard | No explicit `reset()` for post-trip recovery |

#### History

| Issue | Area | Summary |
|-------|------|---------|
| history-serde-json-error-conflation | history | `serde_json` errors wrapped in `AssayError::Io` conflates error categories |
| history-io-error-conflation | history | Same issue, duplicate (overlaps with above) |
| history-list-silent-filter-map | history | `list()` silently drops unreadable directory entries |
| prune-eprintln-in-library | history | `prune()` uses `eprintln!` directly in library code |

### Should-Fix (target: v0.3.0)

Code quality, duplication, naming, and ergonomics improvements.

#### Types

| Issue | Area | Summary |
|-------|------|---------|
| type-invariant-enforcement | types | Domain types accept structurally invalid values (empty cmd, inconsistent kind/exit_code) |
| gaterunrecord-working-dir-should-be-pathbuf | types | `working_dir` should be `Option<PathBuf>` not `Option<String>` |
| types-add-eq-derive | types | Types with `PartialEq` missing `Eq` derive (no floats) |
| enforcement-display-impl | types | `Enforcement` should implement `Display` instead of ad-hoc match |
| feature-spec-minimal-test | types | `FeatureSpec` roundtrip test coverage incomplete |

#### Evaluation

| Issue | Area | Summary |
|-------|------|---------|
| evaluate-all-duplication | evaluation | Structural duplication between `evaluate_all` and `evaluate_all_gates` |
| spec-validation-single-pass | evaluation | Two-pass enforcement validation could be single pass |

#### CLI

| Issue | Area | Summary |
|-------|------|---------|
| cli-spec-cleanup | CLI | Multiple code quality issues (ANSI overhead, specs_dir path, scan exit code, etc.) |
| cli-streamcounters-rename-failed-field | CLI | `failed` field name ambiguous with enforcement tracking |
| cli-streamcounters-gate-blocked-method | CLI | Exit code logic should be encapsulated as method |
| cli-streamcounters-tally-method | CLI | Counter increments should be centralized |
| spec-show-color-branch-duplication | CLI | Color branch duplication in `handle_spec_show` |
| history-status-vs-enforcement | CLI | History table "pass/fail" uses `failed==0` instead of enforcement semantics |

#### MCP

| Issue | Area | Summary |
|-------|------|---------|
| mcp-unnecessary-clones | MCP | `working_dir_owned` clone still exists (partially resolved) |

#### History

| Issue | Area | Summary |
|-------|------|---------|
| history-generate-run-id-visibility | history | `generate_run_id` should be `pub(crate)` not `pub` |
| history-save-error-uses-dir-path | history | Error messages reference directory path instead of temp file path |
| history-list-sort-filename-invariant | history | Sort relies on undocumented filename format invariant |

#### Guard

| Issue | Area | Summary |
|-------|------|---------|
| guard-extract-context-pct-helper | guard | Extract shared context percentage helper (overlaps with must-fix duplication) |
| guard-backup-dir-duplicated | guard | Backup directory path duplicated in soft/hard threshold handlers |

#### Spec/Config

| Issue | Area | Summary |
|-------|------|---------|
| spec-type-refinements | spec | Multiple type design issues from Phase 6 review (tuple types, error categories) |
| error-ergonomics | error | `AssayError` construction lacks ergonomic helpers, empty PathBuf produces misleading errors |
| scan-result-specs-compat | spec | `ScanResult.specs` backward-compat field may be unnecessary |

#### Testing

| Issue | Area | Summary |
|-------|------|---------|
| test-coverage-gaps-phase6 | testing | Missing edge case tests (empty dir partially resolved, others remain) |
| gate-pr-review-suggestions | testing | Gate module PR review: missing pipe error, thread panic, process group tests |

### Nice-to-Have (backlog)

Doc comments, minor tests, cosmetic improvements.

#### Types

| Issue | Area | Summary |
|-------|------|---------|
| gate-section-derive-default | types | `GateSection` should derive `Default` |
| gate-section-module-location | types | `GateSection` arguably belongs in a config module |
| quality-attributes-deny-unknown | types | `QualityAttributes` missing `deny_unknown_fields` |
| gaterunrecord-working-dir-doc-absence | types | `working_dir` field doc doesn't explain when absent |
| max-history-nonzero | types | `max_history` allows `Some(0)`, consider `NonZeroUsize` |
| default-max-history-as-const | types | Default max_history should be associated constant |
| saveresult-derive-eq | types | `SaveResult` should derive `Clone, PartialEq, Eq` |

#### CLI

| Issue | Area | Summary |
|-------|------|---------|
| ansi-overhead-doc-assumption | CLI | `ANSI_COLOR_OVERHEAD` doc should note SGR code assumptions |
| command-separator-hardcoded | CLI | Command column separator uses magic `.repeat(7)` |
| gate-help-duplication | CLI | Gate command help examples duplicated |
| show-status-version-ambiguity | CLI | `show_status` doc should clarify binary vs project version |
| cli-streamconfig-doc-comments | CLI | `StreamConfig` fields lack doc comments |
| cli-streamcounters-doc-comments | CLI | `StreamCounters` fields lack doc comments |
| streamcounters-missing-doc | CLI | `StreamCounters` struct lacks doc comment |
| format-criteria-string-alloc | CLI | `format_criteria_type` allocates String for static literals |
| srs-magic-string-dedup | CLI | `"[srs]"` repeated in 3 places without constant |
| display-ids-double-reverse | CLI | Double-reverse for display_ids could use simpler slice approach |
| history-limit-zero-validation | CLI | `--limit 0` allowed but shows nothing |
| invalid-rust-log-silent-fallback | CLI | No feedback when `RUST_LOG` filter is invalid |

#### Evaluation

| Issue | Area | Summary |
|-------|------|---------|
| validate-gates-spec-doc-stale | evaluation | Doc comment missing enforcement validation rule |
| to-criterion-doc-enforcement | evaluation | Doc should mention enforcement is preserved |
| valid-req-id-undocumented | evaluation | `is_valid_req_id` multi-segment area behavior undocumented |
| file-exists-directory-behavior | evaluation | `FileExists` passes for directories, undocumented |

#### Testing

| Issue | Area | Summary |
|-------|------|---------|
| criterion-path-roundtrip-test | testing | Missing `criterion_with_path` schema roundtrip test |
| evaluate-all-file-exists-failure-test | testing | Missing `evaluate_all` FileExists failure path test |
| file-exists-exit-code-assert | testing | Missing `exit_code == None` assertion for missing file |
| file-exists-subdir-test | testing | Missing subdirectory path resolution test |
| format-gate-response-empty-test | testing | Missing empty results vector test |
| magic-number-truncated-test | testing | 131072 magic number undocumented in truncated test |
| schema-roundtrip-scope-comment | testing | Test should clarify scope vs `spec.validate()` |
| history-regex-lite-match-not-independently-tested | testing | `regex_lite_match` helper untested independently |
| mcp-test-timeout-not-tested | testing | `gate_run_with_timeout` test doesn't test actual timeout |

#### History

| Issue | Area | Summary |
|-------|------|---------|
| history-save-return-pathbuf-leaks-implementation | history | `save()` returning `PathBuf` may leak implementation detail |
| history-load-doc-comment-misdirection | history | `load()` doc describes `deny_unknown_fields` (type-level concern) |

#### MCP / Tooling

| Issue | Area | Summary |
|-------|------|---------|
| ci-plugin-schema-validation | tooling | CI plugin validation only checks JSON syntax, not schema |
| deny-toml-cleanup-todos | tooling | Skip entries should have cleanup TODO comments |
| self-check-fmt-missing-all-flag | tooling | `cargo fmt --check` missing `--all` flag |

#### Design (Future)

| Issue | Area | Summary |
|-------|------|---------|
| phase3-output-detail-enum | types | `OutputDetail` enum for semantic verbosity control |
| phase3-wire-format-types | types | Separate wire (MCP) vs display (CLI) format types |
| phase7-streaming-capture | evaluation | `BufReader` with byte budget for gate evaluation |
| phase8-progressive-disclosure | MCP | Two-tool `gate_run` + `gate_evidence` pattern |
| comment-cleanup-phase3 | types | Comment quality improvements (low priority) |
| tui-use-try-init | TUI | Use `ratatui::try_init()` instead of panicking `init()` |

#### Guard

| Issue | Area | Summary |
|-------|------|---------|
| guard-daemon-derive-debug | guard | Add `Debug` derive to `GuardDaemon` |
| guard-daemon-tracing-instrument | guard | Methods should use `tracing::instrument` |
| guard-circuit-breaker-prune-every-record | guard | `prune_old` called on every record, could batch |
| guard-pid-path-asref | guard | `pid_file_path` could accept `AsRef<Path>` |
| guard-poll-interval-default | guard | Default 5s poll may be aggressive for large sessions |
| guard-status-display | guard | `GuardStatus` could implement `Display` |
| guard-threshold-level-ord | guard | `ThresholdLevel` could implement `Ord` |
| guard-watcher-temp-suffixes-undocumented | guard | Temp file filter suffixes undocumented |

## Duplicate Groups

| Issue A | Issue B | Relationship |
|---------|---------|-------------|
| history-io-error-conflation | history-serde-json-error-conflation | Same root cause: serde_json errors wrapped as Io |
| evaluate-all-duplication | evaluate-gates-duplication | Same refactoring target: shared criterion evaluation helper |
| guard-context-pct-duplicated | guard-extract-context-pct-helper | Same fix: extract shared context percentage helper |
| cli-streamcounters-doc-comments | streamcounters-missing-doc | Overlapping doc comment requests for StreamCounters |

## Summary Statistics

| Priority | Count | Target |
|----------|-------|--------|
| Must-Fix | 31 | v0.2.1 |
| Should-Fix | 22 | v0.3.0 |
| Nice-to-Have | 54 | backlog |
| **Total** | **107** | |

| Area | Must | Should | Nice | Total |
|------|------|--------|------|-------|
| Types | 8 | 5 | 7 | 20 |
| Evaluation | 3 | 2 | 4 | 9 |
| CLI | 2 | 6 | 12 | 20 |
| MCP | 7 | 1 | 1 | 9 |
| Guard | 8 | 2 | 8 | 18 |
| History | 4 | 3 | 2 | 9 |
| Testing | 0 | 2 | 9 | 11 |
| Spec/Config | 0 | 3 | 0 | 3 |
| Tooling/Design | 0 | 0 | 8 | 8 |
| **Total** | **32** | **24** | **51** | **107** |
