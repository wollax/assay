# M002: Real Assay Integration — Research

**Date:** 2026-03-17

## Summary

M002 retires the mock Assay binary and wires Smelt to the real `assay` CLI. The core problem is a three-part contract gap: (1) `AssayInvoker::build_manifest_toml()` emits `[[session]]` (wrong key), embeds an inline description in `spec` (wrong type), and carries harness-like fields that Assay's `RunManifest` doesn't accept; (2) Assay requires a pre-existing `.assay/` project in the container — `assay run` fails immediately without `assay init`; and (3) Assay resolves each `spec` field as a file name in `.assay/specs/`, so Smelt must either write those files itself (Option A) or require them to pre-exist in the host repo (Option B).

**Option A (recommended):** Smelt writes the spec files. Before calling `assay run`, the `AssayInvoker` runs `assay init` in the container (creating `.assay/`), then writes one `<session-name>.toml` spec file per `SessionDef` into `/workspace/.assay/specs/`. The `RunManifest` it generates then simply references the spec names. This keeps Smelt self-contained — users need no `.assay/` directory in their repo — and makes the integration testable in a clean container without any pre-seeded file state. Option B (reference existing specs) is simpler but couples Smelt's manifest format to whatever spec names the host repo already has; it breaks for repos without `.assay/specs/` and gives Smelt nowhere to put the `SessionDef.spec` description text.

The second major concern is test image strategy. Current tests install `git` via `apk add` at runtime; a real `assay` binary isn't on Alpine's CDN and must be compiled from source or copied in as a pre-built binary. The cleanest test approach is to build an `assay` binary from the local source tree and inject it into the container during the test setup phase (following D040's pattern: place it at `/usr/local/bin/assay`). Result collection via `ResultCollector` remains valid — Assay's merge phase updates the host repo (via the bind-mount worktrees), and Smelt can still collect HEAD delta from `base_ref`. No replacement of `ResultCollector` is needed; just ensure `assay init` doesn't interfere with its git ops.

## Recommendation

**Implement Option A** (Smelt generates spec files + manifest). The `AssayInvoker` sequence becomes:
1. `assay init` (or write `.assay/` structure manually) in the container at `/workspace`
2. Write one `Spec` TOML file per session to `/workspace/.assay/specs/<name>.toml`
3. Write a `RunManifest` TOML (with `[[sessions]]` plural, `spec = "<name>"` references)
4. Execute `assay run /tmp/smelt-manifest.toml`

D002 is satisfied: Smelt continues to own its serde structs mirroring Assay's format, with no crate dependency. D029 is superseded — update it with the validated contract.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Writing multiple files into a running container | `AssayInvoker::write_manifest_to_container()` (base64 exec pattern, D028) | Already proven; reuse for spec file writes |
| Building `assay` test binary | `cargo build --bin assay` + `target/debug/assay` | Produces a real binary from the same local source tree; no network needed; already valid for use in tests |
| Mock binary injection at test setup | D040 pattern: `echo base64 \| base64 -d > /usr/local/bin/assay && chmod +x` | Already used in E2E tests; extend to copy real binary instead of mock script |
| Phase-chaining in integration tests | D039 pattern: manual phase calls | Used in `test_full_e2e_pipeline`; avoids `run_with_cancellation()` internals |
| Spec TOML serialization | `toml::to_string_pretty()` | Already a dependency; no new crate needed |

## Existing Code and Patterns

- `crates/smelt-core/src/assay.rs` — `AssayInvoker` to rewrite; `build_manifest_toml()` needs two new helpers: `build_spec_toml(session)` and `write_spec_file_to_container()`; `build_run_command()` also needs: (a) `build_init_command()`, (b) `build_mkdir_specs_command()`; the `AssayManifest`/`AssaySession` structs must be replaced with types matching the real `RunManifest`/`ManifestSession` schema
- `crates/smelt-core/src/manifest.rs` — `SessionDef.spec` (today a free-text description) must remain a description on the Smelt side; the translation to an Assay spec file is done entirely in `AssayInvoker` — no change to `SessionDef` needed for Option A
- `crates/smelt-cli/src/commands/run.rs` — `execute_run()` calls `build_manifest_toml()` + `write_manifest_to_container()` in Phase 6; add a Phase 5.5 that writes spec files and runs init before writing the manifest
- `crates/smelt-cli/tests/docker_lifecycle.rs` — `test_full_e2e_pipeline` uses mock assay placed at `/usr/local/bin/assay` (D040); add a parallel real-assay test that builds `assay` from source and injects the binary
- `crates/smelt-core/src/collector.rs` — `ResultCollector::collect()` reads host git state post-exec; Assay manages its own worktree branches and merges to base; this remains compatible — Smelt reads `HEAD` after `assay run` exits

### Authoritative Assay Contract (from source)

**`RunManifest`** (`assay-types/src/manifest.rs`):
```toml
[[sessions]]
spec = "spec-name"          # references .assay/specs/<spec-name>.toml  (REQUIRED)
name = "optional-display"   # optional
depends_on = ["other"]      # optional
# also: settings, hooks, prompt_layers, file_scope, shared_files
```
- Top key is `sessions` (plural), NOT `session`
- `spec` is a name reference, NOT an inline description
- `deny_unknown_fields` — extra keys like `harness` or `timeout` will parse-fail

**`Spec`** (legacy flat file at `.assay/specs/<name>.toml`):
```toml
name = "spec-name"
description = "Implement the login page"

[gate]
enforcement = "required"

[[criteria]]
name = "tests"
cmd = "npm test -- --filter auth"
```
- `GatesSpec` (directory-based: `.assay/specs/<name>/gates.toml`) is functionally equivalent and also accepted
- Smelt should use the simpler flat `Spec` format to minimize what it writes

**`assay run` flags** (from `assay-cli/src/commands/run.rs`):
- `--timeout <secs>` (default 600) — pass max session timeout
- `--base-branch <branch>` — pass `manifest.job.base_ref`
- `--failure-policy skip-dependents|abort` — exposed via manifest
- `--merge-strategy completion-time|file-overlap`
- `--conflict-resolution auto|skip`

**`assay init`** (from `assay-core/src/init.rs`):
- Creates `.assay/config.toml`, `.assay/specs/`, `.assay/srs.md`, `.assay/.gitignore`
- Fails with `AlreadyInitialized` if `.assay/` already exists — idempotent guard exists
- Can be called as `assay init` CLI command; does NOT need `--name` for Smelt's purposes

**Exit codes** (from `assay-cli/src/commands/run.rs`):
- `0` — all sessions succeeded + merge clean
- `1` — any pipeline error or session failure
- `2` — gate failures / merge conflicts

**Multi-session routing** — `needs_orchestration()` returns true when `sessions.len() > 1` OR any session has `depends_on`. Single-session with no deps uses simpler sequential path.

## Constraints

- **D002 (firm):** No `assay-types` crate dependency in Smelt — replicate serde structs locally; verify against source, don't import
- **D013 (firm):** Bind-mount at `/workspace`; `assay run` needs this as the CWD / project root (Assay reads `.assay/` relative to project root)
- **`deny_unknown_fields` on both sides:** Assay's `RunManifest` and `Spec` both use `deny_unknown_fields`; the TOML Smelt writes must exactly match — no extra fields
- **`assay init` must be idempotent:** If host repo already has `.assay/`, running `assay init` in the container will fail (returns `AlreadyInitialized` error). The init step must tolerate a pre-existing `.assay/` — either check first or write the directory structure manually. Since the bind-mount means any `assay init` runs against the host repo, Smelt should write the files directly (avoiding `assay init` clobbering a real `.assay/`) — or only init if `.assay/` doesn't exist. **This is a critical correctness concern** for repos that already use Assay
- **Assay binary availability:** No published binary; must build from source for integration tests. Pre-build in a CI step or use `cargo build --manifest-path` at test time. The binary doesn't need to be in the container image — it can be injected at test setup
- **`apk add git` fragility:** Existing tests install git at runtime; should be pre-built into the test image or deferred — don't add more `apk add` calls for M002

## Common Pitfalls

- **Writing spec files to wrong path** — `assay run` looks for specs relative to the project root: `.assay/specs/<name>.toml`. The container CWD is `/workspace` (D027), so spec files must go to `/workspace/.assay/specs/`. If they land in `/tmp/.assay/specs/` alongside the manifest, Assay won't find them
- **`assay init` on a repo that already has `.assay/`** — The init returns `AlreadyInitialized` (error). For host repos that already have Assay configured, blindly running `assay init` will error the job. Guard with: check if `/workspace/.assay/config.toml` exists before initiating; only create the `specs/` subdir if missing
- **Forgetting `--base-branch`** — Assay auto-detects HEAD as the base branch if `--base-branch` is not passed. In a container with a bind-mounted repo, HEAD may differ from `manifest.job.base_ref`. Always pass `--base-branch` explicitly
- **Exit code 2 treated as success** — `assay run` exits 2 for gate failures or merge conflicts; Smelt's `run.rs` currently bails on non-zero exits. Exit code 2 should be treated as a distinct result (gates failed), not an error, if Smelt wants to surface partial results
- **Mock assay binary tests becoming stale** — Existing tests in `docker_lifecycle.rs` test the mock path only. The spec TOML the mock validates is the old (`[[session]]`) format; it should be updated even if the mock is retained as a fast-path test
- **`assay run` streaming output** — The current `DockerProvider::exec()` collects all output and returns it after exec completes. Real `assay run` can take many minutes; streaming matters. Research whether bollard's exec API supports streaming stdout/stderr incrementally — the current `ExecHandle` model collects full output only
- **Spec name collision** — Smelt's `SessionDef.name` becomes the spec file name. If a session name contains characters invalid in a TOML filename (e.g. `/`, spaces), the file write or `assay run` spec lookup may fail. Sanitize session names before using them as spec file names

## Open Risks

- **`DockerProvider::exec()` output streaming** — Bollard's exec API does support streaming (AttachContainerOptions + multiplexed stdout/stderr chunks), but `ExecHandle` currently buffers everything. M002's streaming requirement may require a new `exec_streaming()` variant on `RuntimeProvider`, or a second exec method that takes a writer closure. This is a non-trivial API change if done properly
- **`assay init` in the bind-mount context** — Init writes `.assay/.gitignore` and `specs/hello-world.toml` to the host repo. For M002's use case (host repo doesn't have `.assay/`), this permanently modifies the repo. Smelt should not call `assay init` at all — write the `.assay/` structure manually using targeted exec commands, creating only what's needed without the side-effect example files
- **Binary delivery to container** — No production path yet for getting a real `assay` binary into the container at runtime. Options: (a) require users to specify an image with `assay` pre-installed, (b) add an `install_binary` field to the manifest, (c) document that `assay` must be on PATH in the image. Planning must choose one and reflect it in the manifest schema
- **`ResultCollector` and Assay merge** — Assay's orchestration phase checks out the base branch and merges session worktree branches onto it. After `assay run` exits, HEAD on the bind-mount may have moved (if single-session merge happened) or may be on a different branch. The `ResultCollector` reads `HEAD` and compares to `base_ref` — this may see 0 new commits if Assay's merge created no commits beyond what was already on the base. Needs testing with real Assay to confirm behavior

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust / Cargo | — | none found (standard) |
| Docker / bollard | — | none found (project-specific) |
| Assay | — | internal project; no public skill |

## Sources

- `RunManifest` and `ManifestSession` schema with `deny_unknown_fields` (source: `/Users/wollax/Git/personal/assay/crates/assay-types/src/manifest.rs`)
- `Spec` struct — legacy flat spec format used in `.assay/specs/<name>.toml` (source: `/Users/wollax/Git/personal/assay/crates/assay-types/src/lib.rs`)
- `GatesSpec` struct — directory-based spec format (source: `/Users/wollax/Git/personal/assay/crates/assay-types/src/gates_spec.rs`)
- `assay init` behavior: creates `.assay/config.toml`, `specs/`, `srs.md`, `.gitignore` + example spec; returns `AlreadyInitialized` error if `.assay/` already exists (source: `/Users/wollax/Git/personal/assay/crates/assay-core/src/init.rs`)
- `assay run` CLI flags: `--timeout`, `--base-branch`, `--failure-policy`, `--merge-strategy`, `--conflict-resolution`; exit codes 0/1/2; single-session vs orchestrated routing (source: `/Users/wollax/Git/personal/assay/crates/assay-cli/src/commands/run.rs`)
- Spec scanning: both flat `.toml` (legacy) and directory-based (`<name>/gates.toml`) are accepted (source: `/Users/wollax/Git/personal/assay/crates/assay-core/src/spec/mod.rs`)
- `pipeline.rs` loads specs from `specs_dir` relative to project root via `load_spec_entry(&manifest_session.spec, &config.specs_dir)` (source: `/Users/wollax/Git/personal/assay/crates/assay-core/src/pipeline.rs`)
- Existing `AssayInvoker` current (wrong) implementation (source: `crates/smelt-core/src/assay.rs`)
- Existing `docker_lifecycle.rs` mock assay injection pattern — D040/D039 (source: `crates/smelt-cli/tests/docker_lifecycle.rs`)
- Real spec example with `[[criteria]]` `cmd` fields (source: `/Users/wollax/Git/personal/assay/.assay/specs/self-check.toml`)
