---
estimated_steps: 4
estimated_files: 3
---

# T02: LinearTrackerSource bridging LinearClient to TrackerSource

**Slice:** S04 — Linear Tracker Backend
**Milestone:** M012

## Description

Implement `LinearTrackerSource<L: LinearClient>` that bridges the Linear-specific `LinearClient` to the platform-agnostic `TrackerSource` trait. Includes label UUID caching via `ensure_labels()` and the two-mutation `transition_state` implementation (remove old label + add new label).

This mirrors `GithubTrackerSource<G: GhClient>` exactly in structure: constructor, `ensure_labels()`, and `TrackerSource` impl. The key difference is that Linear label mutations require UUIDs (not names), so `ensure_labels()` resolves all lifecycle label name→UUID mappings into a cached `HashMap` at startup.

## Steps

1. **Create `serve/linear/source.rs`** with `LinearTrackerSource<L: LinearClient>`:
   - Fields: `client: L`, `team_id: String`, `label_prefix: String`, `label_cache: HashMap<String, String>` (label name → UUID)
   - Constructor: `new(client, team_id, label_prefix)` with empty label cache

2. **Implement `ensure_labels()`** as `pub async fn`:
   - For each variant in `TrackerState::ALL`, compute `label_name = state.label_name(&self.label_prefix)`
   - Call `client.find_label(&self.team_id, &label_name)` — if found, cache the UUID
   - If not found, call `client.create_label(&self.team_id, &label_name)` — cache the returned UUID
   - Store all mappings in `self.label_cache` (requires `&mut self`)
   - Log with `tracing::info!` on each label ensured

3. **Implement `TrackerSource` for `LinearTrackerSource<L: LinearClient + Send + Sync>`**:
   - `poll_ready_issues()`: compute `ready_label = TrackerState::Ready.label_name(&self.label_prefix)`; call `client.list_issues(&self.team_id, &ready_label)`; map `LinearIssue` → `TrackerIssue` using `identifier` as `id`, `description` as `body`, `url` as `source_url`
   - `transition_state()`: look up both from/to label UUIDs in `label_cache` (error if not found — means `ensure_labels()` wasn't called); parse `issue_id` to find the Linear UUID (research: `issue_id` from poll is `identifier` e.g. "KAT-42", but mutations need the UUID `id` — need to store both); call `client.remove_label(issue_uuid, from_label_uuid)` then `client.add_label(issue_uuid, to_label_uuid)`
   - **Important design note**: `poll_ready_issues` returns `TrackerIssue.id = identifier` (human-readable), but `add_label`/`remove_label` need the UUID. Solution: store a secondary mapping `identifier → uuid` populated during `poll_ready_issues`, OR change `TrackerIssue.id` to use the UUID and put identifier in a display field. Simplest: use the UUID as `TrackerIssue.id` (the TrackerSource consumer only uses `id` as an opaque string for `transition_state`). Put `identifier` in `source_url` context. **Decision: use Linear issue UUID as `TrackerIssue.id`** — it's the natural key for mutations, and consumers treat `id` as opaque.

4. **Write comprehensive unit tests** using `MockLinearClient`:
   - `test_poll_ready_issues_returns_mapped_issues` — verifies `LinearIssue` → `TrackerIssue` mapping
   - `test_poll_ready_issues_empty_result` — no issues returned
   - `test_poll_ready_issues_list_failure` — error propagation
   - `test_transition_state_removes_old_adds_new` — two mutations called
   - `test_transition_state_missing_cache_entry` — error when label not in cache
   - `test_transition_state_remove_failure_propagates` — first mutation fails
   - `test_transition_state_add_failure_propagates` — second mutation fails
   - `test_ensure_labels_creates_missing_labels` — query-then-create path
   - `test_ensure_labels_finds_existing_labels` — query finds existing, no create
   - `test_ensure_labels_populates_cache` — verify cache has all 6 lifecycle labels

## Must-Haves

- [ ] `LinearTrackerSource<L: LinearClient>` implements `TrackerSource`
- [ ] `ensure_labels()` queries first, creates only if not found, caches all label UUIDs
- [ ] `poll_ready_issues()` maps `LinearIssue` to `TrackerIssue` correctly (UUID as id)
- [ ] `transition_state()` calls remove-old then add-new using cached label UUIDs
- [ ] `transition_state()` errors clearly when label cache is missing an entry
- [ ] All 10 unit tests pass

## Verification

- `cargo test -p smelt-cli --lib -- serve::linear::source` — all source tests pass
- `cargo test --workspace` — all tests pass, zero regressions
- `cargo clippy --workspace -- -D warnings` — clean

## Observability Impact

- Signals added/changed: `tracing::info!` on each label ensured (team_id, label_name, action: "found"/"created"); `tracing::info!` on state transition (issue_id, from, to)
- How a future agent inspects this: error messages include `team_id`, label name, and operation context
- Failure state exposed: missing label cache entry produces `SmeltError::tracker("transition", "label 'smelt:queued' not in cache — was ensure_labels() called?")`

## Inputs

- `crates/smelt-cli/src/serve/linear/mod.rs` — LinearClient trait, LinearIssue, LinearLabel from T01
- `crates/smelt-cli/src/serve/linear/mock.rs` — MockLinearClient from T01
- `crates/smelt-cli/src/serve/github/source.rs` — GithubTrackerSource pattern to mirror
- `crates/smelt-core/src/tracker.rs` — TrackerIssue, TrackerState, TrackerState::ALL

## Expected Output

- `crates/smelt-cli/src/serve/linear/source.rs` — LinearTrackerSource with TrackerSource impl + 10 unit tests
- `crates/smelt-cli/src/serve/linear/mod.rs` — Updated with source module registration and re-export
