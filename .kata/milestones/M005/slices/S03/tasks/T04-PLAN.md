---
estimated_steps: 5
estimated_files: 1
---

# T04: Implement `milestone_create` and `spec_create` MCP Tools

**Slice:** S03 — Guided Authoring Wizard
**Milestone:** M005

## Description

Add `milestone_create` and `spec_create` to `AssayServer`, wiring the wizard core functions into the MCP layer. These tools expose programmatic authoring to agent callers (Claude Code, Codex) that cannot use the interactive CLI wizard. Makes the 5 MCP tests written in T01 pass.

## Steps

1. In `crates/assay-mcp/src/server.rs`, add four parameter structs near the existing `CycleAdvanceParams`/`CycleStatusParams` structs:
   ```rust
   #[derive(Deserialize, JsonSchema)]
   pub struct ChunkParams {
       pub slug: String,
       pub name: String,
       pub order: u32,
   }

   #[derive(Deserialize, JsonSchema)]
   pub struct CriterionParams {
       pub name: String,
       pub description: String,
       pub cmd: Option<String>,
   }

   #[derive(Deserialize, JsonSchema)]
   pub struct MilestoneCreateParams {
       pub slug: String,
       pub name: String,
       pub description: Option<String>,
       pub chunks: Vec<ChunkParams>,
   }

   #[derive(Deserialize, JsonSchema)]
   pub struct SpecCreateParams {
       pub slug: String,
       pub name: String,
       pub milestone_slug: Option<String>,
       pub order: Option<u32>,
       pub criteria: Vec<CriterionParams>,
   }
   ```

2. Add `milestone_create()` method to `AssayServer` with `#[tool(description = "Create a milestone TOML in .assay/milestones/ from structured parameters. Creates a Draft milestone with the given slug, name, optional description, and ordered chunk references. Returns the milestone slug on success. Fails if a milestone with the same slug already exists.")]`:
   - `let cwd = resolve_cwd()?;`
   - `let assay_dir = cwd.join(".assay");`
   - Convert `params.0.chunks` to `Vec<(String, u32)>`: `params.0.chunks.iter().map(|c| (c.slug.clone(), c.order)).collect()`.
   - Wrap in `spawn_blocking`: call `assay_core::wizard::create_milestone_from_params(&params.0.slug, &params.0.name, params.0.description.as_deref(), chunks, &assay_dir)`.
   - On success, serialize `milestone.slug` as a JSON string: `serde_json::to_string(&milestone.slug)`.
   - On error, return `domain_error(&e)`.

3. Add `spec_create()` method to `AssayServer` with `#[tool(description = "Create a chunk spec (gates.toml) in .assay/specs/<slug>/ from structured parameters. Optionally links the spec to a milestone by slug and patches the milestone's chunk list. Returns the path to the created gates.toml on success. Fails if the spec directory already exists.")]`:
   - `let cwd = resolve_cwd()?;`
   - Load config for `specs_dir`: `let config = match load_config(&cwd) { Ok(c) => c, Err(e) => return Ok(e) };`
   - `let specs_dir = cwd.join(".assay").join(&config.specs_dir);`
   - `let assay_dir = cwd.join(".assay");`
   - Convert `params.0.criteria` to `Vec<assay_core::wizard::CriterionInput>`.
   - Wrap in `spawn_blocking`: call `assay_core::wizard::create_spec_from_params(&params.0.slug, &params.0.name, params.0.milestone_slug.as_deref(), params.0.order, criteria, &specs_dir, &assay_dir)`.
   - On success, serialize returned `PathBuf` as a JSON string (display).
   - On error, return `domain_error(&e)`.

4. Fill in the 5 MCP tests that T01 added (they had the correct structure but couldn't compile because the params structs/methods were missing). Each test:
   - Uses `create_project()` helper + `std::env::set_current_dir(dir.path())`.
   - `milestone_create_writes_milestone_toml`: call `server.milestone_create(Parameters(MilestoneCreateParams { slug: "test-ms".into(), name: "Test MS".into(), description: None, chunks: vec![ChunkParams { slug: "c1".into(), name: "C1".into(), order: 0 }] }))`, assert `!result.is_error`, assert `dir.path().join(".assay/milestones/test-ms.toml").exists()`.
   - `spec_create_writes_gates_toml`: call `server.spec_create(...)` with slug "chunk-1", no milestone. Assert `!result.is_error`. Assert `dir.path().join(".assay/specs/chunk-1/gates.toml").exists()`.
   - `spec_create_rejects_duplicate`: call `spec_create` twice, assert second `result.is_error.unwrap_or(false)`.

5. Run `cargo test -p assay-mcp -- milestone_create spec_create`, then `cargo test --workspace`, then `just ready`.

## Must-Haves

- [ ] `ChunkParams`, `CriterionParams`, `MilestoneCreateParams`, `SpecCreateParams` all derive `Deserialize, JsonSchema`
- [ ] `milestone_create()` and `spec_create()` use `spawn_blocking` (same pattern as `cycle_advance`)
- [ ] Both methods return `domain_error(&e)` on `AssayError` (no panics, no unwraps in error path)
- [ ] `spec_create` loads config to resolve `specs_dir` (not hardcoded `.assay/specs`)
- [ ] All 5 new MCP tests pass
- [ ] `cargo test --workspace` passes with no regressions (1308+ tests green)
- [ ] `just ready` green

## Verification

```
cargo test -p assay-mcp -- milestone_create
# Expected: 3 tests pass (milestone_create_tool_in_router, milestone_create_writes_milestone_toml, + router test)

cargo test -p assay-mcp -- spec_create
# Expected: 2 tests pass (spec_create_tool_in_router, spec_create_writes_gates_toml, spec_create_rejects_duplicate)

cargo test --workspace
# Expected: 1308+ passed, 0 failed

just ready
# Expected: green
```

## Observability Impact

- Signals added/changed: `milestone_create` returns `isError: true` + `AssayError::Io` message (includes slug) on slug collision; `spec_create` returns `isError: true` on duplicate spec or missing milestone
- How a future agent inspects this: `milestone_create` success → returned slug usable in subsequent `cycle_status` calls; `spec_create` success → returned path confirms file location; both failures are `isError: true` with descriptive message
- Failure state exposed: `domain_error` surfaces `AssayError::Io { operation, path }` — includes operation label ("milestone 'slug' already exists") and path

## Inputs

- `crates/assay-core/src/wizard.rs` — `create_milestone_from_params`, `create_spec_from_params`, `CriterionInput` (produced by T02)
- `crates/assay-mcp/src/server.rs` — existing `cycle_advance` as the reference pattern for `spawn_blocking` + `domain_error`
- `crates/assay-mcp/src/server.rs` test section — `create_project()`, `extract_text()`, `serial`, `AssayServer::new()` helpers already present
- T01 test functions — already defined expected behavior; just needs params structs to compile

## Expected Output

- `crates/assay-mcp/src/server.rs` — `ChunkParams`, `CriterionParams`, `MilestoneCreateParams`, `SpecCreateParams` structs; `milestone_create()` and `spec_create()` methods; 5 passing tests
- `just ready` green with 1308+ workspace tests passing
