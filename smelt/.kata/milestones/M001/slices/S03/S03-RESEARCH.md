# S03: Repo Mount & Assay Execution — Research

**Date:** 2026-03-17

## Summary

S03 adds two capabilities to the existing `DockerProvider`: bind-mounting the host repo into the container and executing `assay run` inside it. The bind-mount path is straightforward — bollard's `HostConfig.binds` accepts `host:container[:options]` strings, and the existing `provision()` already builds a `HostConfig` struct that just needs the `binds` field populated. The Assay invocation is more nuanced: Smelt's `SessionDef` schema diverges significantly from Assay's `RunManifest`/`ManifestSession` schema, so S03 must build a translation layer that converts Smelt sessions into an Assay-compatible manifest TOML, writes it to the container filesystem, and shells out to `assay run`.

The primary risk is the **Assay CLI contract**: Assay's `assay run manifest.toml` expects a `RunManifest` with `[[sessions]]` entries that reference specs by name (specs must exist as `.assay/specs/*.toml` files in the project). Smelt's sessions carry inline `spec` text and `harness` commands, not spec file references. This means S03 either needs to (a) generate Assay spec files + manifest inside the container before invoking `assay run`, or (b) define a mock/shim that stands in for `assay run` to exercise the mount and exec path without a real Assay installation. The roadmap explicitly allows "a mock standing in for it," so the recommended approach is: build the real translation layer shape (`AssayInvoker`) but test against a shell script mock that validates the expected files exist at the mount point and prints structured output.

## Recommendation

**Approach: Real `AssayInvoker` module + mock-based verification**

1. **Extend `DockerProvider::provision()`** to accept bind-mount configuration. Add `binds` to `HostConfig` using `manifest.job.repo` as the host path and a fixed container path (e.g., `/workspace`). The repo path in the manifest is currently a URL string — for local repos, it should be an absolute path; for remote repos, S03 should validate and reject (clone support is deferred).

2. **Create `assay.rs`** with an `AssayInvoker` struct that:
   - Translates `JobManifest` sessions into an Assay-compatible manifest TOML string
   - Writes the manifest to the container via exec (`sh -c 'cat > /tmp/manifest.toml'` or similar)
   - Constructs the `assay run /tmp/manifest.toml --timeout <T>` command
   - Executes via `DockerProvider::exec()` and interprets the result

3. **Wire into `run.rs`** after provision: mount repo → write manifest → exec assay → stream output → teardown.

4. **Test with a mock script** inside an alpine container that validates `/workspace` exists, reads the manifest, and prints structured output. This avoids requiring Assay to be installed in the test image.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Docker bind mounts | `HostConfig.binds` in bollard | Already using `HostConfig` for resources — just add the `binds` field |
| TOML generation | `toml::to_string_pretty()` via serde | Already in workspace deps, Assay manifest is simple TOML |
| Git repo path resolution | `std::fs::canonicalize()` | Bind mounts require absolute host paths; canonicalize resolves symlinks |
| Working directory in exec | `CreateExecOptions.working_dir` | bollard supports setting cwd for exec — use `/workspace` |

## Existing Code and Patterns

- `crates/smelt-core/src/docker.rs` — `provision()` builds `ContainerCreateBody` with `HostConfig { memory, nano_cpus, ..Default::default() }`. Add `binds: Some(vec!["host:container".to_string()])` to this struct. The `exec()` method takes `&[String]` commands and streams stdout/stderr — reuse directly for assay invocation.
- `crates/smelt-core/src/manifest.rs` — `JobManifest.job.repo` is a string field. Currently used only for display in dry-run. S03 must interpret this as a local path for bind-mounting. `SessionDef` has `name`, `spec`, `harness`, `timeout`, `depends_on` — these map to Assay's `ManifestSession` but with different field semantics.
- `crates/smelt-core/src/provider.rs` — `RuntimeProvider::exec()` signature is `(&self, container: &ContainerId, command: &[String])`. This is sufficient for running assay. No trait changes needed.
- `crates/smelt-cli/src/commands/run.rs` — `execute_run()` currently does provision → health-check exec → teardown. S03 replaces the health-check with: write-manifest exec → assay-run exec. The async block cleanup pattern (D026) stays.
- `crates/smelt-core/src/git/cli.rs` — `GitCli` has `repo_root()`, `current_branch()`, `rev_parse()` — useful for resolving the local repo path to an absolute path for bind-mounting.
- Assay's `RunManifest` (in assay-types) — `[[sessions]]` with `spec` (references spec file name), optional `name`, `depends_on`, `settings`, `hooks`, `prompt_layers`. Smelt's inline `spec` text doesn't map to this directly.

## Constraints

- **Bind mounts require absolute host paths.** `manifest.job.repo` must be validated as an absolute local path (or resolved to one). URLs should be rejected with a clear error — clone-into-container is out of scope for M001 (D013 says bind-mount only).
- **bollard exec `working_dir` field.** `CreateExecOptions` has an optional `working_dir: Option<String>` field — set this to `/workspace` so all exec commands run in the mounted repo context.
- **Container filesystem is ephemeral.** The Assay manifest file must be written into the container before `assay run` is invoked. Two options: (a) write via `exec` using `sh -c 'cat << EOF > /tmp/manifest.toml\n...\nEOF'`, or (b) use Docker `put_archive` to push a tar with the file. The exec approach is simpler and uses existing patterns.
- **Assay may not exist in the container image.** The test image is `alpine:3` which doesn't have assay. Tests must use a mock script. Real usage requires an image with assay installed — the manifest's `environment.image` is the user's responsibility (D012).
- **Container runs `sleep 3600` as CMD (D021).** All work happens via exec. Multiple execs in sequence (write manifest, then run assay) work because the container stays alive.
- **Credential env vars are already injected at provision time.** The `provision()` method reads `credentials.env` and passes resolved values as container env vars. Assay inside the container will see these. No additional credential work needed in S03.

## Common Pitfalls

- **Heredoc quoting in exec.** When writing a manifest via `sh -c 'cat > file << EOF\n...\nEOF'`, TOML content with special characters (quotes, brackets) can break shell parsing. Safer to use `echo` with base64 encoding or pipe through `sh -c` with properly escaped content. Best approach: generate the TOML string in Rust, base64-encode it, and decode in the container: `sh -c 'echo <base64> | base64 -d > /tmp/manifest.toml'`.
- **Repo path with spaces or special characters.** Bind mount strings are colon-separated (`host:container`), so colons in paths would break parsing. Validate that the repo path doesn't contain colons. Spaces are fine in bind mount strings.
- **Mount permissions.** The container user must have read/write access to the mounted directory. Alpine's default user is root, so this isn't an issue for testing, but real images may run as non-root. S03 should document this assumption.
- **Assay manifest schema mismatch.** Smelt's `SessionDef.spec` is inline task text, but Assay's `ManifestSession.spec` is a reference to a spec file name. The translation layer must either (a) create spec files in the container's `.assay/specs/` directory, or (b) accept that the mock approach sidesteps this entirely for now. The real integration (S06) will need to solve this properly.
- **Stream interleaving.** Assay's output may be long-running. The existing `exec()` streams stdout/stderr via `eprint!` — this works but mixes with Smelt's own lifecycle messages. Consider prefixing Assay output or using a different stream routing approach in the future.

## Open Risks

- **Assay CLI contract is unstable.** Assay is actively developed (v0.4.0 as of today). The `assay run manifest.toml` command interface, manifest schema, and output format could change. Smelt pins to a specific Assay version via the Docker image, but the translation layer must be kept in sync. Using a mock for S03 tests defers this risk but doesn't eliminate it.
- **Local repo path assumption.** The manifest's `job.repo` field currently accepts any string (URL or path). S03 introduces the first real consumer of this field. The validation logic must distinguish local paths from URLs and reject URLs with a clear message. This is a behavioral change to the manifest semantics.
- **Large repo bind-mount performance.** Bind-mounting a large repo (many GB `.git` directory) could slow container creation. This is a Docker/OS concern, not a bollard concern, but worth noting for user guidance.
- **Assay spec file generation.** The full Assay integration (not just the mock) requires generating `.assay/specs/*.toml` files from Smelt's inline `spec` text. This is a non-trivial translation that may need its own research when S06 approaches. For S03, the mock approach avoids this.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust (general) | `apollographql/skills@rust-best-practices` (2.8K installs) | available — not directly relevant to this slice's Docker/bind-mount focus |
| Rust async | `wshobson/agents@rust-async-patterns` (4.5K installs) | available — patterns already established in S02, not needed |
| Docker | `findinfinitelabs/chuuk@docker-containerization` (7 installs) | available — low install count, unlikely to add value over bollard docs |
| bollard | none found | — |

No skills are recommended for installation. The Docker/bollard patterns are already well-established in the codebase from S02, and the Assay integration is project-specific.

## Sources

- bollard `HostConfig.binds` field accepts `host-src:container-dest[:options]` strings for bind mounts (source: bollard-stubs models.rs, inspected locally)
- bollard `CreateExecOptions` has `working_dir: Option<String>` for setting exec working directory (source: bollard API, inspected locally)
- Assay `RunManifest` schema: `[[sessions]]` with `spec` (file reference), `name`, `depends_on`, `settings`, `hooks`, `prompt_layers` (source: `/Users/wollax/Git/personal/assay/crates/assay-types/src/manifest.rs`)
- Assay CLI: `assay run manifest.toml --timeout <N> [--json] [--base-branch <B>]` (source: `/Users/wollax/Git/personal/assay/crates/assay-cli/src/commands/run.rs`)
- Assay orchestrator uses DAG-driven parallel execution with `depends_on` for session ordering (source: `/Users/wollax/Git/personal/assay/crates/assay-core/src/orchestrate/executor.rs`)
