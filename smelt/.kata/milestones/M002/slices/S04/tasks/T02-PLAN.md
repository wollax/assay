---
estimated_steps: 4
estimated_files: 2
---

# T02: Add merge-commit and serde unit tests to collector.rs and monitor.rs

**Slice:** S04 — Exit Code 2 + Result Collection Compatibility
**Milestone:** M002

## Description

Two unit tests that close the explicit verification gaps left by S04's research:

1. **`test_collect_after_merge_commit`** in `collector.rs`: proves `ResultCollector::collect()` handles Assay's post-merge state correctly. Assay merges session branches to the base branch inside the container; the bind-mount means those merge commits land on the host repo. `collect()` must see the merge commit as a new commit ahead of the base and create the target branch at HEAD. This test creates the scenario locally with `git merge --no-ff` and verifies the collector handles it without error.

2. **`test_job_phase_gates_failed_serde`** in `monitor.rs`: proves the new `GatesFailed` variant round-trips through TOML serde correctly — `"gates_failed"` → `JobPhase::GatesFailed` → `"gates_failed"`. Since `run-state.toml` is the primary observability surface, silent serde corruption of the new variant would be a regression risk.

No production code changes — tests only.

## Steps

1. Open `crates/smelt-core/src/collector.rs`. In the `#[cfg(test)]` module, add `test_collect_after_merge_commit` after the last existing test. Use the existing `setup_test_repo()` and `add_commit()` helpers.

   Test body:
   ```rust
   #[tokio::test]
   async fn test_collect_after_merge_commit() {
       let (tmp, cli) = setup_test_repo();
       let base = head_hash(tmp.path());
       let git_bin = which::which("git").expect("git on PATH");
       let run = |args: &[&str]| {
           let out = std::process::Command::new(&git_bin)
               .args(args)
               .current_dir(tmp.path())
               .output()
               .expect("git command");
           assert!(out.status.success(), "git {} failed: {}", args.join(" "), String::from_utf8_lossy(&out.stderr));
       };

       // Create a feature branch with one commit, then merge it back with --no-ff.
       // This mirrors Assay's behavior: Assay merges session branches to base inside the container.
       run(&["checkout", "-b", "feat"]);
       add_commit(tmp.path(), "session-output.txt", "gate results", "session: gate passed");
       run(&["checkout", "-"]);  // back to default branch
       run(&["merge", "--no-ff", "feat", "-m", "merge session results"]);

       let collector = ResultCollector::new(cli, tmp.path().to_path_buf());
       let result = collector.collect(&base, "results/after-merge").await.unwrap();

       assert!(!result.no_changes, "merge commit should be detected as new commits");
       assert_eq!(result.commit_count, 2, "feat commit + merge commit = 2");
       assert_eq!(result.branch, "results/after-merge");
       assert!(!result.files_changed.is_empty(), "merged files must appear in diff");

       // Branch should point at current HEAD (the merge commit).
       let current_head = head_hash(tmp.path());
       let branch_hash = {
           let out = std::process::Command::new(&git_bin)
               .args(["rev-parse", "results/after-merge"])
               .current_dir(tmp.path())
               .output()
               .unwrap();
           String::from_utf8_lossy(&out.stdout).trim().to_string()
       };
       assert_eq!(branch_hash, current_head, "target branch must point at merge commit HEAD");
   }
   ```

2. Open `crates/smelt-core/src/monitor.rs`. Add a `#[cfg(test)]` module at the bottom (if one doesn't already exist). Add `test_job_phase_gates_failed_serde`:

   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_job_phase_gates_failed_serde() {
           // Serialize GatesFailed → must produce "gates_failed"
           let serialized = toml::to_string(&JobPhase::GatesFailed).unwrap();
           assert!(
               serialized.contains("gates_failed"),
               "expected 'gates_failed' in serialized output, got: {serialized}"
           );

           // Deserialize "gates_failed" → must round-trip back to GatesFailed
           let deserialized: JobPhase = toml::from_str(&serialized).unwrap();
           assert_eq!(deserialized, JobPhase::GatesFailed);
       }
   }
   ```

   Note: `toml::to_string` for an enum variant serializes as a bare string (e.g. `"gates_failed"\n`). If `monitor.rs` doesn't already have `toml` available as a dev dependency (it's in `smelt-core/Cargo.toml` as a regular dep), the test can use it directly.

3. Run `cargo test -p smelt-core test_collect_after_merge_commit -- --nocapture` and `cargo test -p smelt-core test_job_phase_gates_failed_serde -- --nocapture` individually.

4. Run `cargo test --workspace` to confirm total pass count increased and no regressions.

## Must-Haves

- [ ] `test_collect_after_merge_commit` creates a `--no-ff` merge commit, calls `collect()`, and asserts `commit_count == 2`, `!no_changes`, and branch points at HEAD
- [ ] `test_job_phase_gates_failed_serde` serializes `GatesFailed` and confirms `"gates_failed"` in output; deserializes back and confirms round-trip
- [ ] Both tests pass without any production code changes
- [ ] `cargo test --workspace` shows at least 112 passed (was 110 before S04; T01 adds 0 unit tests; T02 adds 2)

## Verification

```bash
# New merge-commit test
cargo test -p smelt-core test_collect_after_merge_commit -- --nocapture
# Expected: test result: ok. 1 passed

# New serde test
cargo test -p smelt-core test_job_phase_gates_failed_serde -- --nocapture
# Expected: test result: ok. 1 passed

# Full workspace — confirm total count increased
cargo test --workspace 2>&1 | grep "test result"
# Expected: all "test result: ok." with smelt-core showing 112 passed (or more)
```

## Observability Impact

- Signals added/changed: `test_job_phase_gates_failed_serde` makes the `"gates_failed"` serde mapping an explicit, tested contract — future changes to the enum will break this test first rather than silently corrupting run-state.toml
- How a future agent inspects this: `cargo test -p smelt-core -- --nocapture` shows both new test names in output; `grep test_collect_after_merge_commit crates/smelt-core/src/collector.rs` confirms presence
- Failure state exposed: If `collect()` fails on a merge commit scenario (e.g. rev_list_count or diff_name_only behaves differently with merge parents), `test_collect_after_merge_commit` surfaces the exact assertion failure with `--nocapture` git output

## Inputs

- `crates/smelt-core/src/collector.rs` — `setup_test_repo()`, `add_commit()`, `head_hash()` helpers and existing test structure to extend
- `crates/smelt-core/src/monitor.rs` — `JobPhase::GatesFailed` variant added in T01 (must be present before this test can compile)
- T01 output: `GatesFailed` variant in `monitor.rs` — required for the serde test to reference

## Expected Output

- `crates/smelt-core/src/collector.rs` — `test_collect_after_merge_commit` added to `#[cfg(test)]` module
- `crates/smelt-core/src/monitor.rs` — `test_job_phase_gates_failed_serde` added to `#[cfg(test)]` module (create module if absent)
