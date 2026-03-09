---
phase: 28
plan: 1
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/assay-types/src/lib.rs
  - crates/assay-types/src/worktree.rs
  - crates/assay-core/src/lib.rs
  - crates/assay-core/src/error.rs
  - crates/assay-core/src/worktree.rs
autonomous: true
must_haves:
  truths:
    - "WorktreeConfig, WorktreeInfo, and WorktreeStatus are serializable types in assay-types"
    - "Config struct has an optional worktree field for [worktree] TOML section"
    - "AssayError has WorktreeGit, WorktreeGitFailed, WorktreeExists, WorktreeNotFound, and WorktreeDirty variants"
    - "assay_core::worktree module exposes create, list, status, cleanup functions that shell out to git CLI"
    - "Worktree create validates spec exists before creating"
    - "Worktree list parses git worktree list --porcelain and filters to assay/ branches"
    - "Worktree status reports branch, dirty state, and ahead/behind counts"
    - "Worktree cleanup removes worktree and deletes associated branch"
    - "Config precedence: worktree_dir argument > ASSAY_WORKTREE_DIR env > config file > default sibling dir"
    - "detect_main_worktree() returns the main repo path when running inside a linked worktree (ORCH-07)"
  artifacts:
    - path: "crates/assay-types/src/worktree.rs"
      provides: "WorktreeConfig, WorktreeInfo, WorktreeStatus serializable structs"
    - path: "crates/assay-types/src/lib.rs"
      provides: "Config.worktree optional field, pub mod worktree re-export"
    - path: "crates/assay-core/src/error.rs"
      provides: "5 Worktree error variants on AssayError"
    - path: "crates/assay-core/src/worktree.rs"
      provides: "Git worktree lifecycle functions: create, list, status, cleanup, resolve_worktree_dir, detect_main_worktree"
  key_links:
    - from: "assay-types WorktreeConfig"
      to: "assay-core worktree::resolve_worktree_dir"
      via: "Config.worktree.base_dir feeds into path resolution"
    - from: "assay-core worktree::create"
      to: "assay-core spec::load_spec_entry"
      via: "Spec existence validated before worktree creation"
    - from: "assay-core error.rs (WorktreeGit, WorktreeGitFailed)"
      to: "assay-core worktree.rs git_command helper"
      via: "All git shell-outs use these error variants"
---

<objective>
Implement the foundation layer for worktree management: serializable types in assay-types, error variants in assay-core, and the core worktree module with git CLI integration. This provides create/list/status/cleanup functions that the CLI and MCP layers will consume.

Purpose: ORCH-01 through ORCH-04, ORCH-06, ORCH-07 — git worktree lifecycle with configurable paths and spec resolution from worktree context.
Output: WorktreeConfig/WorktreeInfo/WorktreeStatus types, 5 error variants, core worktree module with 6 public functions.
</objective>

<execution_context>
<!-- Executor agent has built-in instructions -->
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/phases/pending/28-worktree-manager/28-CONTEXT.md
@.planning/phases/pending/28-worktree-manager/28-RESEARCH.md
@crates/assay-types/src/lib.rs
@crates/assay-core/src/lib.rs
@crates/assay-core/src/error.rs
@crates/assay-core/src/spec/mod.rs
</context>

<tasks>
<task type="auto">
  <name>Task 1: Worktree types and error variants</name>
  <files>
    - crates/assay-types/src/worktree.rs
    - crates/assay-types/src/lib.rs
    - crates/assay-core/src/error.rs
  </files>
  <action>
    1. Create `crates/assay-types/src/worktree.rs` with three types:

       **WorktreeConfig** — configuration for [worktree] TOML section:
       - `base_dir: String` with `#[serde(default)]` (empty string default; resolved at runtime using project name)
       - Derive: Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema
       - `#[serde(deny_unknown_fields)]`
       - Submit to schema_registry

       **WorktreeInfo** — information about a created/listed worktree:
       - `spec_slug: String`, `path: PathBuf`, `branch: String`, `base_branch: String`
       - Derive: Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema

       **WorktreeStatus** — extends WorktreeInfo with runtime state:
       - `spec_slug: String`, `path: PathBuf`, `branch: String`, `head: String`, `dirty: bool`, `ahead: usize`, `behind: usize`
       - Derive: Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema

    2. In `crates/assay-types/src/lib.rs`:
       - Add `pub mod worktree;` to the module declarations
       - Add re-exports: `pub use worktree::{WorktreeConfig, WorktreeInfo, WorktreeStatus};`
       - Add optional `worktree` field to `Config` struct:
         ```rust
         #[serde(default, skip_serializing_if = "Option::is_none")]
         pub worktree: Option<WorktreeConfig>,
         ```

    3. In `crates/assay-core/src/error.rs`:
       - Add 5 error variants to `AssayError` (see 28-RESEARCH.md for exact definitions):
         - `WorktreeGit { cmd: String, source: std::io::Error }` — git command failed to spawn
         - `WorktreeGitFailed { cmd: String, stderr: String, exit_code: Option<i32> }` — git exited non-zero
         - `WorktreeExists { spec_slug: String, path: PathBuf }` — worktree already exists
         - `WorktreeNotFound { spec_slug: String }` — no worktree for spec
         - `WorktreeDirty { spec_slug: String }` — uncommitted changes, cleanup refused
       - Follow existing error format patterns (see GateExecution, SpecNotFound for style reference)
  </action>
  <verify>
    cargo check -p assay-types 2>&1 | tail -5
    cargo check -p assay-core 2>&1 | tail -5
    cargo test -p assay-types 2>&1 | tail -10
  </verify>
  <done>
    - WorktreeConfig, WorktreeInfo, WorktreeStatus compile and derive all required traits
    - Config struct accepts optional [worktree] section
    - All 5 error variants exist on AssayError with correct #[error] messages
    - Existing tests pass
  </done>
</task>

<task type="auto">
  <name>Task 2: Core worktree module with git CLI integration</name>
  <files>
    - crates/assay-core/src/worktree.rs
    - crates/assay-core/src/lib.rs
  </files>
  <action>
    1. Create `crates/assay-core/src/worktree.rs` with the following public functions:

       **git_command helper** (private):
       - Takes `args: &[&str]` and `cwd: &Path`
       - Spawns `git` via `Command`, captures stdout/stderr
       - Returns `Result<String, AssayError>` using WorktreeGit/WorktreeGitFailed variants
       - Trims trailing whitespace from stdout

       **resolve_worktree_dir** (public):
       - Signature: `pub fn resolve_worktree_dir(cli_override: Option<&str>, config: &Config, project_root: &Path) -> PathBuf`
       - Precedence: cli_override > env var ASSAY_WORKTREE_DIR > config.worktree.base_dir > default
       - Default: `project_root/../{project_name}-worktrees/`
       - Resolve relative paths against project_root
       - Do NOT canonicalize (path may not exist yet)

       **detect_default_branch** (private):
       - Runs `git symbolic-ref refs/remotes/origin/HEAD` and strips prefix
       - Falls back to "main" on failure

       **create** (public):
       - Signature: `pub fn create(project_root: &Path, spec_slug: &str, base_branch: Option<&str>, worktree_base: &Path, specs_dir: &Path) -> Result<WorktreeInfo>`
       - Validate spec exists: check `specs_dir.join(format!("{spec_slug}.toml"))` exists, else return `SpecNotFound`
       - Compute worktree_path = `worktree_base.join(spec_slug)`
       - If worktree_path already exists, return `WorktreeExists` error
       - Create worktree_base dir if needed (`create_dir_all`)
       - Resolve base_branch: parameter > detect_default_branch
       - Branch name: `assay/{spec_slug}`
       - Run: `git worktree add -b assay/{spec_slug} {worktree_path} {base_branch}`
       - Return WorktreeInfo

       **list** (public):
       - Signature: `pub fn list(project_root: &Path, worktree_base: &Path) -> Result<Vec<WorktreeInfo>>`
       - Run `git worktree prune` first (cleanup stale entries)
       - Run `git worktree list --porcelain`
       - Parse porcelain output using parse_worktree_list helper (see research)
       - Filter to entries whose branch starts with `assay/`
       - Derive spec_slug by stripping `assay/` prefix from branch name
       - Return Vec<WorktreeInfo> sorted by spec_slug

       **status** (public):
       - Signature: `pub fn status(worktree_path: &Path, spec_slug: &str) -> Result<WorktreeStatus>`
       - If worktree_path doesn't exist, return WorktreeNotFound
       - Get branch: `git -C {path} rev-parse --abbrev-ref HEAD`
       - Get HEAD sha: `git -C {path} rev-parse --short HEAD`
       - Check dirty: `git -C {path} status --porcelain` (non-empty = dirty)
       - Get ahead/behind: `git -C {path} rev-list --left-right --count HEAD...@{upstream}` — parse "N\tM", default 0/0 on failure (no upstream)
       - Return WorktreeStatus

       **cleanup** (public):
       - Signature: `pub fn cleanup(project_root: &Path, worktree_path: &Path, spec_slug: &str, force: bool) -> Result<()>`
       - If worktree_path doesn't exist, return WorktreeNotFound
       - Check dirty state: `git -C {path} status --porcelain`
       - If dirty and !force, return WorktreeDirty
       - Run `git worktree remove [--force] {path}` (add --force if force=true OR dirty)
       - Run `git branch -D assay/{spec_slug}` (force-delete the branch, ignore error if branch doesn't exist)
       - Return Ok(())

       **detect_main_worktree** (public):
       - Signature: `pub fn detect_main_worktree(cwd: &Path) -> Option<PathBuf>`
       - Check if `.git` is a file (linked worktree) vs directory (main worktree)
       - If file: parse `gitdir: <path>`, navigate to main repo root
       - If directory: return None (already in main worktree)
       - See research for exact implementation

    2. In `crates/assay-core/src/lib.rs`:
       - Add `pub mod worktree;` declaration

    3. Add unit tests in the module:
       - `test_parse_worktree_list_normal` — standard porcelain output with 2 worktrees
       - `test_parse_worktree_list_empty` — empty input returns empty vec
       - `test_parse_worktree_list_bare` — bare worktree entry is handled (no branch)
       - `test_parse_worktree_list_detached` — detached HEAD entry is handled
       - `test_resolve_worktree_dir_default` — uses sibling directory when no overrides
       - `test_resolve_worktree_dir_config` — config.worktree.base_dir is used
       - `test_resolve_worktree_dir_env_overrides_config` — env var takes precedence

    4. Add integration tests that use real git repos:
       - Create temp dir, `git init`, create a spec file
       - Test create (verify worktree and branch exist)
       - Test list (verify created worktree appears)
       - Test status (verify clean state)
       - Test cleanup (verify worktree and branch removed)
       - Test create with nonexistent spec returns SpecNotFound
       - Test cleanup of dirty worktree without force returns WorktreeDirty
       - Use `tempfile::TempDir` for isolation
  </action>
  <verify>
    cargo check -p assay-core 2>&1 | tail -5
    cargo test -p assay-core worktree 2>&1 | tail -30
    just lint 2>&1 | tail -10
  </verify>
  <done>
    - All 6 public functions compile and pass clippy
    - Unit tests for parsing and config resolution pass
    - Integration tests with real git repos pass (create, list, status, cleanup)
    - Error cases covered: nonexistent spec, dirty cleanup without force, worktree already exists
    - `just ready` passes
  </done>
</task>
</tasks>

<verification>
```bash
just ready
cargo test -p assay-core worktree -- --nocapture 2>&1 | tail -30
```
</verification>

<success_criteria>
- [ ] WorktreeConfig, WorktreeInfo, WorktreeStatus types exist in assay-types with full serde/schemars derives
- [ ] Config struct accepts optional `[worktree]` TOML section
- [ ] 5 Worktree error variants exist on AssayError
- [ ] assay_core::worktree module exposes create, list, status, cleanup, resolve_worktree_dir, detect_main_worktree
- [ ] Config precedence: cli_override > env var > config file > default sibling directory
- [ ] Spec existence validated before worktree creation
- [ ] parse_worktree_list unit tests pass
- [ ] Integration tests with real git repos pass
- [ ] `just ready` passes
</success_criteria>

<output>
After completion, create `.planning/phases/28-worktree-manager/28-01-SUMMARY.md`
</output>
