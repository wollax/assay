# Pitfalls Research: v0.7.0 Gate Composability

**Date:** 2026-04-11
**Scope:** Common mistakes when adding gate composability primitives (`gate.extends`, criteria libraries with `include`, spec preconditions) and a guided wizard across CLI/MCP/TUI to the existing Assay v0.6.x codebase (2266 tests, 57 binaries, zero-trait convention).

**Supersedes:** v0.3.0 PITFALLS.md items P-41 through P-65 (those pitfalls are addressed or out of scope for v0.7.0).

---

## 1. Schema Evolution Pitfalls

### P-66: `deny_unknown_fields` on `GatesSpec` and `Criterion` breaks existing TOML files when new fields are added

**Area:** Schema / Backward Compatibility
**Confidence:** High (directly verified in codebase — both `GatesSpec` and `Criterion` carry `#[serde(deny_unknown_fields)]`)

**What goes wrong:** `GatesSpec` and `Criterion` both use `#[serde(deny_unknown_fields)]`. Adding `extends: Option<String>` to `GatesSpec` or `include: Vec<String>` to a new library type is safe for NEW files. But the real risk is the reverse: the new fields must use `#[serde(default)]` or they will cause deserialization failures on any file that was written by a previous version of Assay (which, of course, doesn't contain those fields yet).

More subtle: if `gate.extends` is added to `GatesSpec` but the corresponding entry in the existing `[[criteria]]` TOML block uses a field name that conflicts with an already-used key (e.g., a criterion accidentally named `extends`), the new discriminator silently wins.

The existing codebase already set a precedent: `milestone`, `order`, `depends`, and `when` all carry `#[serde(default)]` / `skip_serializing_if` — this is the correct pattern and must be extended to every composability field.

**Warning signs:**
- Test deserializing pre-v0.7.0 fixture files against the new types fails
- Existing users get "missing field" or "unknown field" errors after upgrading Assay
- The schema snapshot test (`just schemas`) diverges without a corresponding migration

**Prevention:**
- Every new field on `GatesSpec` or `Criterion` must carry `#[serde(default, skip_serializing_if = "...")]`
- Add a backward-compat roundtrip test: embed a pre-v0.7.0 TOML fixture (no composability fields) and assert it deserializes cleanly and re-serializes without emitting the new fields
- Run `just schemas` and commit the schema snapshot diff alongside the type change — reviewers can see exactly what changed
- Do not add `extends` or `include` to `Criterion` itself — confine new composability fields to `GatesSpec` only (criteria are resolved before evaluation, so criteria never need to reference other criteria by name)

**Which phase should address it:** The first composability phase (gate inheritance via `gate.extends`). Every subsequent phase that adds a new field must follow the same pattern.

---

### P-67: Criteria library files need their own TOML type — reusing `GatesSpec` silently admits invalid states

**Area:** Schema / Type Safety
**Confidence:** High (architectural analysis of the proposed `include` mechanism)

**What goes wrong:** The natural implementation of a criteria library is to store shared criteria in a TOML file and reference it with `include = ["lib/baseline.toml"]`. If the library file is represented as a `GatesSpec` (or deserialized into one), it inherits all `GatesSpec` fields: `name`, `gate` section, `depends`, `milestone`, `order`. Most of those fields are meaningless in a library — a library doesn't have a milestone, doesn't have a gate enforcement section, isn't a spec. If they're present in a library file, they should be an error; if absent, they bloat the schema.

Reusing `GatesSpec` also means library files can contain `extends` themselves, creating multi-level include chains that are complex to resolve and cycle-detect.

**Warning signs:**
- Library TOML files accepted with `milestone = "something"` (which has no effect but is misleading)
- Users confused about whether a library file "counts" as a spec (it doesn't)
- Multi-level include chains that require recursive resolution logic

**Prevention:**
- Define a separate `CriteriaLibrary` type in `assay-types` with only `name: String`, `description: String`, and `criteria: Vec<Criterion>`
- Library files live in a dedicated directory (e.g., `.assay/libraries/`) and are NOT loadable as specs
- Validation: `CriteriaLibrary` uses `#[serde(deny_unknown_fields)]` — reject `extends`, `depends`, `milestone` in library files
- Limit include depth to 1: a `GatesSpec` can include a library, but a library cannot include another library (avoids transitive resolution complexity)
- Library resolution happens at load time, before validation — the resolved spec (with criteria merged in) is what gets validated

**Which phase should address it:** The criteria library phase. Must be decided before any implementation begins.

---

## 2. Inheritance and Resolution Pitfalls

### P-68: Inheritance cycles via `gate.extends` are not detected by the existing cycle-detection code

**Area:** Gate Inheritance / Cycle Detection
**Confidence:** High (existing cycle detection in `spec/validate.rs` operates on `depends` fields, not on `extends`)

**What goes wrong:** The existing `detect_cycles()` in `spec/validate.rs` performs DFS over the `depends` graph. The new `gate.extends` field creates a second, independent inheritance graph. A spec can form a cycle: `spec-a.toml` has `extends = "spec-b"` and `spec-b.toml` has `extends = "spec-a"`. If the resolver naively follows `extends` pointers without cycle detection, it will loop infinitely or stack-overflow.

Additionally, a spec could `extends` a spec that also has `depends` relationships. The two graphs (dependency graph and inheritance graph) are distinct and can form cycles independently or jointly.

**Warning signs:**
- Stack overflow during spec loading (infinite recursion in the extends resolver)
- "thread 'main' has overflowed its stack" panic during `assay gate run`
- CI hangs indefinitely loading specs

**Prevention:**
- Apply the same DFS/color-marking algorithm from `detect_cycles()` to the `extends` graph, separately
- Detect mixed cycles: if `extends` resolution would require loading a spec that is already being resolved (on the DFS stack), emit an error immediately
- Limit `extends` to a single level (a spec can extend one parent; the parent cannot extend another) — this eliminates transitive cycles entirely at the cost of some flexibility. Revisit if users request multi-level inheritance
- Validate extends targets exist before running cycle detection (a missing target is a distinct error from a cycle)
- Add the extends graph validation to `spec_validate` MCP tool so agents catch this early

**Which phase should address it:** Gate inheritance phase (`gate.extends`), before any resolution logic is written.

---

### P-69: Field override semantics for inherited criteria are ambiguous

**Area:** Gate Inheritance / Merge Semantics
**Confidence:** High (design decision with significant implementation consequences)

**What goes wrong:** When a child spec `extends` a parent, both may define criteria. The merge semantics are undefined: does the child's criterion with the same name override the parent's? Does it append? Does a child criterion with the same name as a parent criterion error out? Each choice produces different user experience and has different implementation complexity.

Ambiguous semantics lead to spec authors being surprised by which criteria actually run. The existing `Criterion` struct has no unique ID field — it uses `name` as a natural key, but names are not enforced as unique within a spec (validation warns on duplicates, but doesn't error).

**Warning signs:**
- Users discover that inherited criteria silently didn't run because their name matched a child criterion
- Two criteria with the same name both appear in the evaluated set (double-run)
- No documentation on override semantics leads to each user assuming different behavior

**Prevention:**
- Define override semantics explicitly before implementing: recommended approach is "child criteria with the same name fully replace parent criteria; child criteria with unique names are appended after parent criteria"
- Make override semantics visible: in `gate run` output, annotate inherited criteria with their source (`inherited from parent-spec`)
- Enforce criterion name uniqueness within a resolved (post-merge) spec — emit an error if the merge would produce duplicates and the user didn't explicitly override
- Add the override semantics to the `GatesSpec` type doc comment and the TOML schema description

**Which phase should address it:** Gate inheritance phase, in the design step before implementation.

---

### P-70: `include` library resolution order affects criterion evaluation order

**Area:** Criteria Libraries / Evaluation Order
**Confidence:** Medium (depends on implementation choice)

**What goes wrong:** A spec with `include = ["lib/security", "lib/performance"]` must merge two library's criteria into the spec's own criteria list. The order in which the merged criteria appear determines the order they run (and the order they appear in gate results). If the resolution merges in an undefined order (e.g., HashMap iteration order), evaluation order becomes non-deterministic. Users who rely on criterion ordering for correctness (e.g., "compile before test") will see flaky gate runs.

**Warning signs:**
- Criteria from included libraries appear in different orders across different runs
- A fast-failing criterion (e.g., "compile check") runs after slower criteria, wasting time
- Gate result reports are hard to compare across runs because criterion order varies

**Prevention:**
- Define resolution order explicitly: included libraries are inserted before the spec's own criteria, in declaration order of the `include` array
- Use `Vec<CriteriaLibrary>` (ordered) not `HashMap` for library resolution
- Document the order in the `GatesSpec` schema: "criteria are evaluated as: [library-1 criteria] ++ [library-2 criteria] ++ [spec's own criteria]"
- Add a test that asserts criterion order after resolution matches declaration order

**Which phase should address it:** Criteria library phase.

---

## 3. Precondition Pitfalls

### P-71: Spec preconditions that fail silently leave gates in an ambiguous state

**Area:** Spec Preconditions
**Confidence:** High (gate evaluation already has `AlwaysPass` and enforcement levels, but no "blocked" state)

**What goes wrong:** The proposed preconditions feature adds conditions that must hold before a spec's gates can run. The current `GateResult` has `passed: bool` — it's binary. A precondition failure is neither a pass nor a gate failure in the traditional sense: it means "evaluation was blocked, not attempted". If a blocked precondition is represented as `passed: false`, it pollutes history with false failures. If it's silently skipped, the user doesn't know why gates didn't run.

The existing `EnforcementSummary` (in `assay-types`) tracks required/advisory pass/fail counts but has no concept of "skipped due to precondition".

**Warning signs:**
- Gate run reports 0 criteria evaluated with no explanation
- History shows a "failed" run when the failure was actually a precondition block
- CI pipeline fails the gate run when a precondition isn't met, even if this was expected

**Prevention:**
- Add a `Skipped` variant (or `blocked_by_precondition: bool` field) to `GateResult` or introduce a distinct `PreconditionResult` type
- Emit an explicit diagnostic: "Gate run skipped — precondition `<name>` not met: <reason>"
- Distinguish precondition failures from gate failures in `GateRunSummary` — add a `preconditions_failed: Vec<String>` field
- Preserve existing behavior: if no preconditions are declared, gate run proceeds exactly as today (zero regression)
- The `gate_run` MCP tool should return a distinct status code for "blocked by precondition" vs "gates ran and failed"

**Which phase should address it:** Spec preconditions phase.

---

### P-72: Preconditions that shell out to commands create a circular evaluation dependency

**Area:** Spec Preconditions / Evaluation
**Confidence:** Medium (design smell that becomes a real problem if not constrained)

**What goes wrong:** If preconditions are expressed as arbitrary shell commands (the same way gate criteria use `cmd`), users will reach for complex preconditions: "check that the database is running", "check that the previous spec's gates passed", "check that a file exists". The last case is fine (maps to `GateKind::FileExists`). The middle case — "check that another spec's gates passed" — creates a dependency between gate evaluations that the current system has no way to manage. If preconditions can reference other specs' gate results, evaluation order becomes a scheduling problem.

**Warning signs:**
- Precondition that runs `assay gate run other-spec` (recursive invocation)
- Precondition that reads gate history files directly
- Users creating DAG-like precondition chains that Assay can't order automatically

**Prevention:**
- Constrain preconditions to only two kinds initially: `FileExists` checks and `EnvVar` checks
- Explicitly do NOT allow arbitrary shell commands in preconditions in v0.7.0 — this is separate from criteria (which do allow commands)
- Document the constraint: "preconditions are declarative checks, not executable scripts"
- If "previous spec must pass" is needed, use the existing `depends` field (which already models spec ordering) rather than preconditions

**Which phase should address it:** Spec preconditions phase, in the design step.

---

## 4. Wizard Pitfalls

### P-73: Three-surface wizard with shared core diverges when surfaces add local state

**Area:** Wizard / Cross-Surface Consistency
**Confidence:** High (directly observed — existing `wizard.rs` is already pure, but surfaces will add interactive state)

**What goes wrong:** The existing `wizard.rs` module is pure: `create_from_inputs`, `create_milestone_from_params`, and `create_spec_from_params` take explicit parameters and write files. This is the correct pattern. The pitfall is that when CLI (adds interactive `dialoguer` prompts), TUI (adds multi-step pane state), and MCP (adds JSON parameter validation) each add their own layer, they drift. A validation rule added to the CLI wizard (e.g., "name must not contain slashes") doesn't get added to the MCP tool, so agents can create specs with invalid names.

**Warning signs:**
- The same validation error is caught in CLI but silently accepted via MCP
- TUI wizard allows blank criteria names that CLI rejects
- Test coverage exists for CLI wizard but not for MCP tool parameter validation
- Users report that `spec_create` MCP tool creates invalid specs that `assay spec show` fails to load

**Prevention:**
- All validation lives in `wizard.rs` (the shared core), not in surface-specific code. Surface code calls `wizard::validate_inputs(&inputs)?` before calling `wizard::create_from_inputs`
- Define a `WizardValidationError` type (or extend `AssayError`) that all surfaces can return
- Write unit tests for `wizard::validate_inputs` independent of any surface
- CLI, MCP, and TUI surfaces are only responsible for collecting inputs and presenting results — no business logic
- Add a surface parity test: create the same spec via CLI path and MCP path, assert the written files are identical

**Which phase should address it:** Wizard implementation phase. Must be established before any surface-specific wizard code is written.

---

### P-74: MCP wizard tool uses positional/optional parameters inconsistently with other MCP tools

**Area:** Wizard / MCP Tool Design
**Confidence:** High (existing MCP tool design decision: additive tools, no optional param changes to existing tools)

**What goes wrong:** The existing MCP tools (`spec_get`, `gate_run`, `session_create`, etc.) use explicit required parameters with no positional arguments. The wizard for gate definitions needs to accept potentially complex inputs: parent gate name (for `extends`), library references (for `include`), precondition declarations, and a list of criteria. Cramming all of this into a single `gate_wizard` MCP tool with many optional parameters creates interfield validation ambiguity — the same problem the `Additive orchestrate_* MCP tools` decision was made to avoid.

**Warning signs:**
- `gate_wizard` MCP tool has 15+ parameters, half of which are optional and interdependent
- Agent calling the tool needs to guess which parameters are required vs. optional for a given use case
- Tool validation errors are confusing: "either `extends` or `criteria` must be provided, but not both in certain configurations"

**Prevention:**
- Split wizard actions into discrete tools: `gate_create` (new gate from scratch), `gate_extend` (create a gate that inherits from a parent), `gate_apply_library` (add a library include to an existing spec)
- Each tool has a minimal, unambiguous parameter set
- Follow the existing precedent: no parameter is both optional AND conditionally required based on other parameters
- Provide a higher-level `plan_gate_wizard` tool that drives the step-by-step flow (asking one question at a time, returning next-step prompts) — this suits how agents naturally operate

**Which phase should address it:** Wizard implementation phase, in MCP tool schema design.

---

### P-75: Interactive CLI wizard breaks non-TTY invocations (CI, scripting)

**Area:** Wizard / CLI Surface
**Confidence:** High (well-known `dialoguer`/`inquire` pitfall)

**What goes wrong:** The CLI wizard (using `dialoguer` or `inquire` for interactive prompts) blocks waiting for TTY input. When run in a CI environment (no TTY), the process hangs or panics. Users who script Assay (e.g., `assay plan new-gate | tee output.txt`) also break because the wizard prompt can't be piped.

The existing CLI already has `NO_COLOR` and TTY detection patterns (referenced in v0.3.0 requirements) — this precedent must be extended to the wizard.

**Warning signs:**
- CI pipeline hangs indefinitely at "Enter gate name:" prompt
- `assay plan` exits with panic on `isatty() == false`
- Scripted invocations (e.g., via `expect` or Makefile) fail

**Prevention:**
- Detect TTY at wizard entry: `if !std::io::stdout().is_terminal() { return Err(...) }` with a clear error message: "interactive wizard requires a TTY; use `assay plan gate create --name foo --cmd 'cargo test'` for scripted use"
- Provide all wizard flows as non-interactive equivalents via CLI flags (`--name`, `--extends`, `--include`, etc.)
- The non-interactive CLI path calls `wizard::create_from_inputs` directly (same code path as the interactive wizard after input collection)
- Test both TTY and non-TTY code paths in CI using `std::io::IsTerminal` mock or pipes

**Which phase should address it:** Wizard implementation phase (CLI surface).

---

### P-76: Wizard editing existing gate definitions requires TOML round-trip fidelity

**Area:** Wizard / Edit Mode
**Confidence:** High (TOML serialization ordering is non-deterministic in some crate versions)

**What goes wrong:** The wizard supports not just creating new gates but editing existing ones. Editing requires: read the TOML, present it for modification, write back. The `toml` crate serializes keys in struct field declaration order, not in the order they appeared in the original file. Comments (inline or block) are not preserved. If a user has manually organized their `gates.toml` (grouped criteria by category, added inline comments), the wizard edit will strip all comments and reorder fields.

This is not just UX friction — it generates noisy git diffs that obscure the actual change.

**Warning signs:**
- `git diff` after wizard edit shows dozens of lines changed with no semantic difference
- User comments in `gates.toml` are lost after wizard edit
- Criteria reordering appears in diffs confusing code reviewers

**Prevention:**
- Treat editing as "show current values as defaults, write a new file" — clearly document that comments are not preserved
- For the v0.7.0 scope: prefer creating new gates over editing existing ones in the wizard; editing can be a follow-on feature
- If editing is required: use a line-oriented patch strategy (find the TOML key, replace its value) rather than full deserialize-modify-reserialize
- Display a warning before overwriting: "This will reformat `gates.toml`. Comments will not be preserved."
- Consider `toml_edit` crate (which preserves whitespace and comments) instead of `toml` for wizard writes to existing files

**Which phase should address it:** Wizard implementation phase. Decide edit vs. create-new scope before implementation.

---

## 5. Enum Dispatch Pitfalls

### P-77: Adding composability metadata to `GateKind` enum variants is the wrong layer

**Area:** Enum Dispatch / Architecture
**Confidence:** High (design smell — `GateKind` is an evaluation type, not a definition type)

**What goes wrong:** `GateKind` (in `assay-types/src/gate.rs`) describes HOW a gate is evaluated at runtime. `GatesSpec` (in `assay-types/src/gates_spec.rs`) describes WHAT criteria exist in a spec. The composability features (`extends`, `include`, preconditions) belong to the definition layer (`GatesSpec`), not the evaluation layer (`GateKind`). The pitfall is adding a new `GateKind::Inherited` variant or a `GateKind::FromLibrary` variant to represent inherited criteria at evaluation time — this contaminates the evaluation type with definition-layer concerns.

If `GateKind` gains a variant for "this came from a library", the match arms in `gate/mod.rs:evaluate()` must handle it, the `Display` impl must handle it, the MCP serialization must handle it, and all existing tests must be updated.

**Warning signs:**
- A new `GateKind::Inherited { .. }` variant is added to represent where a criterion came from
- The `evaluate()` match arm for the new variant delegates to another `GateKind` recursively
- Existing `gate_kind_unknown_variant_deser_fails` tests must be updated to not reject the new variant

**Prevention:**
- Composability (extends, include, preconditions) is fully resolved at load time, before evaluation
- By the time `evaluate()` is called, the `Criterion` it receives has already been fully materialized — all inheritance and library includes have been flattened into concrete `cmd`/`path`/`kind` fields
- `GateKind` never needs a new variant for composability features
- Validate this invariant with a test: after resolving a spec with `extends`, assert that no `GateKind` variant contains any reference to the parent spec

**Which phase should address it:** Gate inheritance phase, as an explicit architectural constraint.

---

### P-78: Criterion deduplication after merge produces name collisions that fail silently

**Area:** Enum Dispatch / Criterion Merging
**Confidence:** Medium (subtle correctness issue)

**What goes wrong:** When a child spec extends a parent, and both define a criterion named `"tests"`, the merge produces one criterion (child overrides parent — the intended behavior from P-69). But if the merge is implemented naively with `parent_criteria.extend(child_criteria)`, both criteria appear in the list. If `evaluate_all()` iterates the list without deduplication, both run. If the child criterion fails but the parent criterion passes, the parent's passing result could win (depending on how results are aggregated).

The existing `Criterion` struct has no explicit deduplication by name — the codebase assumes criteria lists are pre-validated for uniqueness.

**Warning signs:**
- Gate run reports more criteria evaluated than the spec declares
- A criterion appears twice in gate results with different pass/fail status
- The "faster" duplicate wins the aggregation, hiding a real failure

**Prevention:**
- Implement merge as: `let merged = merge_criteria(parent, child)` where the function builds an `IndexMap<name, Criterion>`, inserting parent criteria first, then child criteria (child overrides via map insertion)
- Preserve insertion order of the map for deterministic evaluation order
- Add a post-merge validation step that asserts criterion names are unique in the resolved spec
- Test: create a parent with `["compile", "test"]` and a child with `["test", "lint"]`; assert merged result is `["compile", "test" (from child), "lint"]`

**Which phase should address it:** Gate inheritance phase.

---

## 6. Cross-Cutting Pitfalls

### P-79: `spec_validate` MCP tool does not validate composability references at call time

**Area:** MCP / Validation
**Confidence:** High (the existing `spec_validate` tool validates a single spec in isolation)

**What goes wrong:** The existing `spec_validate` MCP tool (`validate_gates_spec()` in `spec/validate.rs`) validates a single spec: criterion fields, command existence, dependency format. With composability, a spec referencing `extends = "base-gate"` or `include = ["lib/security"]` requires cross-file validation: do the referenced files exist? Do they contain valid criteria? Are there cycles? A single-spec validator cannot answer these questions.

If agents call `spec_validate` and receive a clean bill of health on a spec with an invalid `extends` reference, they will be surprised when `gate run` fails with "extends target not found".

**Warning signs:**
- `spec_validate` returns no errors on a spec with `extends = "nonexistent-spec"`
- Agent creates a spec via wizard, validates it, then runs gates — gates fail with resolution error
- Resolution errors only surface at runtime, not at validation time

**Prevention:**
- Add a `resolve: bool` flag to `spec_validate` (default `true`) that triggers composability resolution and validates the full resolved spec
- When `resolve: true`, the validator loads referenced parent specs and libraries, resolves them, and validates the merged result
- Emit a `Severity::Error` diagnostic when an `extends` target or library `include` doesn't exist
- Emit a `Severity::Error` diagnostic when cycle detection finds an inheritance cycle
- The existing `detect_cycles()` must be extended to cover the `extends` graph (see P-68)

**Which phase should address it:** Each composability phase should include a `spec_validate` extension alongside the feature.

---

### P-80: TOML schema snapshot tests must be updated for each new composability field

**Area:** Schema Registry / Testing
**Confidence:** High (schema snapshot tests already exist — new fields will cause them to fail)

**What goes wrong:** The project uses `inventory::submit!` to register schema entries and `just schemas` to generate JSON schema snapshots. These snapshots are checked into the repo and fail CI if they drift. Adding `extends: Option<String>` to `GatesSpec` changes the JSON schema, causing `just schemas` to produce a different output. If the developer forgets to run `just schemas` and commit the updated snapshot, CI fails.

This is a known friction point: the developer must remember to run `just schemas` for every type change, or the PR is blocked.

**Warning signs:**
- CI fails with "schema snapshots out of date" after adding a new field
- The PR adds a type change but no schema snapshot update
- Schema snapshot diffs are large and confuse reviewers (many unrelated changes appear)

**Prevention:**
- Run `just schemas` as part of the standard development loop for this milestone (add it to the gate criteria for v0.7.0 specs)
- The schema snapshot diff IS the documentation of the public API change — review it as such
- For large composability changes, consider staging: add the field to the type in one commit (updating schema), implement the resolution logic in a subsequent commit

**Which phase should address it:** Every phase that adds new type fields. Note in phase requirements: "run `just schemas` and commit snapshot update."

---

### P-81: Wizard creates files outside `.assay/` when `specs_dir` or `assay_dir` is not validated

**Area:** Wizard / Path Safety
**Confidence:** Medium (path traversal via user input)

**What goes wrong:** The wizard accepts user-provided names that get slugified and used as directory names. The existing `validate_path_component()` (called in `create_from_inputs`, `create_spec_from_params`) rejects slugs containing path separators, but the composability wizard has new attack surfaces: the `extends` target name and library `include` paths are also user-provided and will be used to construct file paths for resolution.

A user (or agent) providing `extends = "../../../etc/passwd"` would cause the resolver to attempt to open an arbitrary file. The existing slug validation doesn't apply to `extends` values (which reference existing spec names, not new directory names).

**Warning signs:**
- `extends` or `include` values containing `..` or `/` are accepted without error
- Resolution code uses string concatenation to build paths from user-provided values
- A spec can load arbitrary files outside `.assay/specs/` by crafting an `extends` reference

**Prevention:**
- Validate all `extends` and `include` values as slug-format: lowercase alphanumeric and hyphens only (`^[a-z0-9][a-z0-9-]*$`)
- Resolution builds paths by: `specs_dir.join(slug)` — `join` on an absolute path replaces the base, so assert the resolved path starts with `specs_dir`
- Use `Path::starts_with()` to assert containment after joining
- Add a test: `extends = "../../outside"` should fail with a validation error, not a file read

**Which phase should address it:** Gate inheritance phase (first use of `extends` as a path component).

---

## Summary by Phase

| Phase | Pitfalls | Critical |
|-------|----------|----------|
| Gate inheritance (`gate.extends`) | P-66, P-68, P-69, P-77, P-78, P-79, P-80, P-81 | P-66, P-68, P-77 |
| Criteria libraries (`include`) | P-67, P-70, P-79, P-80 | P-67, P-70 |
| Spec preconditions | P-71, P-72, P-79 | P-71, P-72 |
| Wizard (shared core) | P-73, P-80 | P-73 |
| Wizard (CLI surface) | P-75, P-76 | P-75 |
| Wizard (MCP surface) | P-74, P-79 | P-74 |
| Wizard (TUI surface) | P-73, P-76 | P-73 |
| Cross-cutting | P-66, P-79, P-80, P-81 | P-66, P-81 |
