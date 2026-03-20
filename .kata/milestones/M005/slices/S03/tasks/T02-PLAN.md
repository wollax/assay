---
estimated_steps: 6
estimated_files: 2
---

# T02: Implement `assay-core::wizard` Module

**Slice:** S03 — Guided Authoring Wizard
**Milestone:** M005

## Description

Implement the `assay-core::wizard` module as a set of pure functions with no TTY dependency. The wizard module is the shared foundation that both the CLI (`assay plan`) and the MCP tools (`milestone_create`, `spec_create`) call. It writes real TOML files to disk using the established atomic write patterns.

## Steps

1. Create `crates/assay-core/src/wizard.rs`. Define types:
   - `pub struct CriterionInput { pub name: String, pub description: String, pub cmd: Option<String> }`
   - `pub struct ChunkInput { pub name: String, pub criteria: Vec<CriterionInput> }` — chunk slug is derived from `slugify(&name)` inside `create_from_inputs`
   - `pub struct WizardInputs { pub name: String, pub description: Option<String>, pub chunks: Vec<ChunkInput> }` — milestone slug derived from `slugify(&name)`
   - `pub struct WizardResult { pub milestone_path: PathBuf, pub spec_paths: Vec<PathBuf> }`

2. Implement `pub fn slugify(s: &str) -> String`: lowercase the string, replace any run of chars matching `[^a-z0-9]` with a single `-`, strip leading/trailing hyphens. Validate result with `validate_path_component` — panic if empty after trimming (slugify is for user-provided names, not untrusted input; the validation in callers provides the user-visible error).

3. Implement `pub fn create_from_inputs(inputs: &WizardInputs, assay_dir: &Path, specs_dir: &Path) -> Result<WizardResult>`:
   - Derive `milestone_slug = slugify(&inputs.name)` and call `validate_path_component(&milestone_slug, "milestone slug")`.
   - Check for slug collision: if `assay_dir.join("milestones").join(format!("{milestone_slug}.toml")).exists()` → return `AssayError::Io { operation: format!("milestone '{milestone_slug}' already exists"), ... }`.
   - Build `Milestone { slug: milestone_slug.clone(), name: inputs.name.clone(), description: inputs.description.clone(), status: MilestoneStatus::Draft, chunks: vec_of_chunk_refs, completed_chunks: vec![], depends_on: vec![], pr_branch: None, pr_base: None, created_at: Utc::now(), updated_at: Utc::now() }`.
   - Call `milestone_save(assay_dir, &milestone)`.
   - For each chunk (enumerate with index `i`): derive `chunk_slug = slugify(&chunk.name)`, build `GatesSpec { name: chunk_slug.clone(), description: String::new(), gate: None, depends: vec![], milestone: Some(milestone_slug.clone()), order: Some(i as u32), criteria: vec_of_gate_criteria }`. Serialize with `toml::to_string_pretty()`. Write atomically to `specs_dir/<chunk_slug>/gates.toml` using `NamedTempFile::new_in` + `write_all` + `sync_all` + `persist` (same pattern as `milestone_save`). Create `specs_dir/<chunk_slug>/` with `create_dir_all` first.
   - Return `WizardResult { milestone_path: final milestone path, spec_paths: vec of gates.toml paths }`.

4. Implement `pub fn create_milestone_from_params(slug: &str, name: &str, description: Option<&str>, chunks: Vec<(String, u32)>, assay_dir: &Path) -> Result<Milestone>`:
   - Validate slug via `validate_path_component`.
   - Check slug collision (same as above).
   - Build `Milestone` with `chunks: Vec<ChunkRef>` from the input tuples (slug + order).
   - Call `milestone_save(assay_dir, &milestone)`.
   - Return the `Milestone` struct.

5. Implement `pub fn create_spec_from_params(slug: &str, name: &str, milestone_slug: Option<&str>, order: Option<u32>, criteria: Vec<CriterionInput>, specs_dir: &Path, assay_dir: &Path) -> Result<PathBuf>`:
   - Validate slug via `validate_path_component`.
   - Check spec dir doesn't already exist: `if specs_dir.join(slug).exists() { return AssayError::Io collision }`.
   - If `milestone_slug` is `Some(ms)`: call `milestone_load(assay_dir, ms)` — return `AssayError::Io` if not found, letting the existing error propagate.
   - Build `GatesSpec { name: slug, milestone: milestone_slug.map(str::to_string), order, criteria: ... }`. Serialize and write atomically.
   - If `milestone_slug` is `Some(ms)`: reload milestone, push `ChunkRef { slug: slug.to_string(), order: order.unwrap_or(0) }` to `chunks`, call `milestone_save(assay_dir, &updated_milestone)`.
   - Return the path to the written `gates.toml`.

6. Add `pub mod wizard;` to `crates/assay-core/src/lib.rs`. Run `cargo test -p assay-core --features assay-types/orchestrate --test wizard` and confirm all 5 tests pass.

## Must-Haves

- [ ] `CriterionInput`, `ChunkInput`, `WizardInputs`, `WizardResult` structs are `pub` and in `wizard.rs`
- [ ] `slugify()` produces `"my-feature"` from `"My Feature!"` and `"my-feature-2"` from `"My Feature 2"`
- [ ] `create_from_inputs()` calls `milestone_save()` (atomic) and writes each `gates.toml` atomically via `NamedTempFile`
- [ ] Each generated `GatesSpec` has `milestone: Some(milestone_slug)` and `order: Some(i as u32)`
- [ ] `create_from_inputs()` rejects slug collision with a clear `AssayError::Io` message
- [ ] `create_spec_from_params()` rejects non-existent milestone with `AssayError::Io` (propagated from `milestone_load`)
- [ ] `create_spec_from_params()` patches the milestone's `chunks` Vec when `milestone_slug` is provided
- [ ] `pub mod wizard;` added to `lib.rs`
- [ ] `GatesSpec` struct construction uses explicit field syntax (not `..Default::default()` — `GatesSpec` has no `Default` impl); all fields must be set

## Verification

```
cargo test -p assay-core --features assay-types/orchestrate --test wizard
# Expected: 5 passed, 0 failed

cargo test -p assay-core --features assay-types/orchestrate
# Expected: all existing core tests still pass (no regression)
```

## Observability Impact

- Signals added/changed: `create_from_inputs()` returns `WizardResult` with all created paths — callers can surface these to users; `AssayError::Io` with operation label on every failure
- How a future agent inspects this: `assay milestone list` shows the wizard-created milestone; `assay spec list` shows the created specs; `assay gate run <slug>` validates gates.toml was well-formed
- Failure state exposed: slug collision error message includes the slug; spec dir collision error message includes the path; milestone-not-found error from `milestone_load` includes the path

## Inputs

- `crates/assay-core/src/milestone/mod.rs` — `milestone_save(assay_dir, &milestone)` and `milestone_load(assay_dir, slug)` function signatures
- `crates/assay-core/src/history/mod.rs` — `pub(crate) fn validate_path_component(value: &str, label: &str) -> Result<()>` (accessible within the same crate)
- `crates/assay-types/src/milestone.rs` — `Milestone`, `ChunkRef`, `MilestoneStatus` types
- `crates/assay-types/src/gates_spec.rs` — `GatesSpec` type with `milestone` and `order` fields (from S01)
- `crates/assay-core/tests/wizard.rs` — T01 test file defines required function signatures and behaviors
- `crates/assay-core/src/init.rs` — `InitResult` pattern (`created_files: Vec<PathBuf>`) and atomic write pattern reference
- `crates/assay-cli/src/commands/spec.rs:handle_spec_new()` — authoritative `gates.toml` template format reference

## Expected Output

- `crates/assay-core/src/wizard.rs` — complete wizard module with 4 public functions and 4 public types
- `crates/assay-core/src/lib.rs` — `pub mod wizard;` line added
- All 5 tests in `crates/assay-core/tests/wizard.rs` passing
