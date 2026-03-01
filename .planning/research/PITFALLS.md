# Pitfalls Research: v0.1.0 Vertical Slice

**Date:** 2026-02-28
**Scope:** Common mistakes when adding MCP server, TOML spec parsing, command gate evaluation, config loading, Claude Code plugin integration, and error types to the Assay Rust workspace.

---

## Critical Pitfalls

### P-01: Stdout corruption in MCP stdio transport

**Area:** MCP Server (rmcp)
**What goes wrong:** Any output written to stdout that is not a valid JSON-RPC message breaks the MCP protocol. The stdio transport reserves stdout exclusively for protocol messages. This includes `println!()` debug statements, tracing/log output defaulting to stdout, and panic messages that write to stdout.

**Why it's easy to miss:** Rust's default tracing subscriber writes to stdout. During development, `println!()` debugging is second nature. The server compiles and runs fine, but the MCP client silently drops the connection or returns parse errors.

**Prevention:**
- Initialize tracing subscriber with `.with_writer(std::io::stderr)` and `.with_ansi(false)` before any other initialization.
- Add a project-level clippy lint or code review check banning `println!()` in the MCP server crate.
- Test the MCP server binary by piping its stdout through a JSON validator as a smoke test.
- Consider a wrapper that redirects stdout at the fd level as a safety net.

**Assay-specific risk:** The CLI and MCP server share `assay-core`. If core functions use `println!()` for user-facing output (reasonable in CLI context), those same functions corrupt the MCP transport when called from the server. Core must never write to stdout directly; output should be returned as data.

---

### P-02: Pipe buffer deadlock in gate subprocess evaluation

**Area:** Gate Evaluation (std::process::Command)
**What goes wrong:** When capturing both stdout and stderr from a gate command via `Stdio::piped()`, the parent blocks on `wait()` while the child blocks trying to write to a full pipe buffer. The pipe buffer is typically 4KB-64KB depending on the OS. Any gate command producing moderate output (e.g., a test suite, linter) triggers this.

**Why it's easy to miss:** Small test commands produce little output and work fine. The deadlock only appears with real workloads where stdout/stderr exceed the buffer size. On macOS, the default pipe buffer is 64KB, so it surfaces later than on Linux (where it can be as low as 4KB in some configurations).

**Prevention:**
- Use `Command::output()` (which reads pipes before waiting) instead of `Command::spawn()` + `child.wait()`.
- If streaming output is needed, spawn dedicated threads to drain stdout and stderr concurrently before calling `wait()`.
- Set explicit output size limits in GateResult (truncate at N bytes) to bound memory usage.
- Document the deadlock risk in gate evaluation code with a comment explaining why `output()` is used.

**Assay-specific risk:** Gate evaluation captures stdout/stderr as evidence in GateResult. The natural approach of `spawn()` + reading stdout + reading stderr + `wait()` is exactly the deadlock pattern. The `output()` method is the correct default for v0.1.

---

### P-03: Async/sync boundary collision between MCP server and gate evaluation

**Area:** MCP Server + Gate Evaluation
**What goes wrong:** rmcp requires a tokio async runtime (`#[tokio::main]`, async tool handlers). Gate evaluation uses synchronous `std::process::Command`. Calling blocking `Command::output()` directly inside an async tool handler blocks the tokio worker thread, starving other tasks. Under a single-threaded runtime (including `#[tokio::test]`), this causes a deadlock.

**Why it's easy to miss:** With `#[tokio::main]` (multi-threaded), blocking a single worker may not visibly stall the server during light testing. It surfaces under load or in tests using `#[tokio::test]` (single-threaded runtime).

**Prevention:**
- Wrap all `std::process::Command` calls in `tokio::task::spawn_blocking()` when called from async context.
- Keep gate evaluation functions in `assay-core` synchronous (taking no async dependency). The MCP server crate is responsible for bridging via `spawn_blocking()`.
- Never use `futures::executor::block_on()` inside a tokio runtime; it takes over the worker thread and can deadlock.
- Test MCP tool handlers with `#[tokio::test(flavor = "multi_thread")]` to catch threading issues.

**Assay-specific risk:** The v0.1 design calls for "sync gate evaluation" with "no async in core." This is correct, but the MCP server (which must be async due to rmcp) needs to call those sync functions. The bridging layer in the MCP server crate is the critical point.

---

### P-04: Zombie processes from gate commands that ignore timeouts

**Area:** Gate Evaluation (std::process::Command)
**What goes wrong:** `std::process::Command` has no built-in timeout. If a gate command hangs (infinite loop, waiting on network, blocked on input), the parent process blocks indefinitely on `wait()` or `output()`. If the process is killed but not waited on, it becomes a zombie. If `Child` is dropped without `wait()`, the child process continues running as an orphan.

**Why it's easy to miss:** Rust's `Child` struct has no `Drop` implementation that kills the process. Dropping it silently leaks the child. Gate commands during development are fast, so timeouts aren't tested.

**Prevention:**
- Implement a timeout wrapper using `Child::try_wait()` in a polling loop with `thread::sleep()`, or use a dedicated thread with a timeout channel.
- Always call `child.kill()` followed by `child.wait()` on timeout. The `wait()` after `kill()` is necessary to reap the zombie.
- Consider the `wait-timeout` crate or similar for cleaner timeout semantics.
- Set a reasonable default timeout (e.g., 300 seconds) with override in spec/gate config.
- When wrapping `Command::output()` in `spawn_blocking()` for the MCP server, the timeout must be on the blocking task itself, not just the process.

**Assay-specific risk:** Gate evaluation captures evidence for GateResult. A runaway gate blocks the entire evaluation pipeline. With the MCP server calling gate evaluation, a hung gate also blocks the MCP tool response, which may cause the MCP client (Claude Code) to time out and retry or abandon.

---

## Moderate Pitfalls

### P-05: TOML enum deserialization does not support external tagging

**Area:** TOML Spec Parsing
**What goes wrong:** Serde's default enum representation (externally tagged, e.g., `{ "Command": { ... } }`) does not work with the TOML format. TOML cannot represent the nested table structure that externally tagged enums require. Attempting to deserialize an externally tagged enum from TOML produces confusing errors about expected types.

**Why it's easy to miss:** The same struct with `#[derive(Deserialize)]` works perfectly with JSON but fails with TOML. Serde examples predominantly use JSON, creating a false expectation.

**Prevention:**
- Use `#[serde(tag = "kind")]` (internally tagged) or `#[serde(untagged)]` for enums that will be deserialized from TOML.
- For `GateKind`, prefer internally tagged: `#[serde(tag = "kind")]` with variants like `kind = "command"`. This is readable in TOML and explicit.
- Avoid `#[serde(untagged)]` for enums with structurally similar variants; it tries each variant in order and returns the error from the last attempt, making debugging impossible.
- Test deserialization from TOML strings in unit tests, not just from JSON.

**Assay-specific risk:** The domain model uses `GateKind` as an enum (command, and future: file, threshold, agent-evaluated). If the enum uses serde's default external tagging, spec files will be unparseable. This must be decided before any spec file format is published.

---

### P-06: Missing or extra TOML fields produce unhelpful errors

**Area:** Config Loading / TOML Spec Parsing
**What goes wrong:** The `toml` crate's error messages for missing required fields or unknown fields are minimal. A missing required field says `missing field 'x'` without indicating which table or line. Extra fields are silently ignored by default (serde's behavior), meaning typos in field names go undetected.

**Why it's easy to miss:** Happy-path testing always has correct fields. Typos and missing fields surface only when users write specs by hand.

**Prevention:**
- Use `#[serde(deny_unknown_fields)]` on config and spec structs to catch typos. This is critical for user-authored TOML files.
- Implement a validation layer on top of deserialization: parse first (toml -> struct), then validate semantically (non-empty names, valid paths, etc.).
- Wrap `toml::from_str()` errors with file path and context: "Error parsing spec file 'path/to/spec.toml': missing field 'name' in [gate]".
- Use `#[serde(default)]` intentionally, not reflexively. Every default field is a field the user can silently omit.

**Assay-specific risk:** Spec files are the primary user-authored artifact. Poor error messages here directly impact the core user experience. The "trim-then-validate" pattern from STATE.md requirements means validation is a two-step process: deserialization tolerates whitespace, then validation checks semantics.

---

### P-07: thiserror `#[from]` creates implicit conversion chains across crate boundaries

**Area:** Error Type Design
**What goes wrong:** Using `#[from]` on error variants creates `From<InnerError>` implementations. When `assay-core` has `#[from] toml::de::Error` and also `#[from] std::io::Error`, any function returning `Result<T, CoreError>` can implicitly convert both error types. This hides the origin of errors, making it hard to distinguish "config file not found" (io::Error) from "config file malformed" (toml::de::Error) at the call site.

**Why it's easy to miss:** The `?` operator with `#[from]` feels ergonomic. The problem is invisible until you need to match on error variants and realize multiple code paths produce the same variant.

**Prevention:**
- Use `#[from]` sparingly. Prefer explicit error construction: `toml::from_str(s).map_err(AssayError::ConfigParse)?`.
- Create domain-specific error variants, not passthrough wrappers: `ConfigParse { path: PathBuf, source: toml::de::Error }` instead of `Toml(#[from] toml::de::Error)`.
- Add context (file path, operation description) at the conversion site, not in the error type.
- For a workspace, keep `#[non_exhaustive]` off error types in `assay-types` (they are internal DTOs), but use it on public error types in `assay-core` if the crate is ever published.

**Assay-specific risk:** STATE.md says "add error variants when consumed, not speculatively." This is correct but creates pressure to add `#[from]` variants lazily. Each `#[from]` should be a conscious decision about whether the conversion preserves enough context.

---

### P-08: rmcp tool handler parameter schema mismatch with schemars

**Area:** MCP Server (rmcp)
**What goes wrong:** rmcp uses `schemars::JsonSchema` to generate the parameter schema for MCP tools. If the parameter struct's schemars-generated schema diverges from what serde actually accepts (e.g., due to custom deserializers, `#[serde(with)]`, or flatten), the MCP client sends parameters that match the schema but fail deserialization. The error surfaces as "failed to deserialize parameters" at runtime.

**Why it's easy to miss:** schemars and serde are separate derive macros with different code paths. Most simple structs work identically, but divergence appears with: `#[serde(flatten)]`, `#[serde(with = "...")]` (schemars requires the `with` target to implement `JsonSchema`), `Option<T>` with `#[serde(default)]` vs `#[serde(skip_serializing_if)]`, and untagged enums.

**Prevention:**
- Keep tool parameter structs simple: flat fields, no `#[serde(flatten)]`, no custom serializers.
- Derive both `JsonSchema` and `Deserialize` on all parameter types and verify roundtrip in tests.
- For `assay-types` DTOs reused as MCP parameters, ensure both `schemars` and `serde` attributes are compatible.
- Generate JSON schemas as part of CI (the `just schemas` pipeline) and diff them to catch regressions.

**Assay-specific risk:** The v0.1 MCP tools are `spec/get` and `gate/run`. Their parameters likely reuse or mirror types from `assay-types`. Any schemars/serde mismatch between the types crate and the MCP parameter handling will surface as runtime deserialization failures that the schema alone can't diagnose.

---

### P-09: `CLAUDE_PLUGIN_ROOT` resolution and binary path issues

**Area:** Claude Code Plugin Integration
**What goes wrong:** The `.mcp.json` file in a Claude Code plugin uses `${CLAUDE_PLUGIN_ROOT}` for relative paths to the server binary. If the server binary isn't built, isn't at the expected path, or lacks execute permissions, the plugin fails silently or with an unhelpful "Connection closed" error. Claude Code doesn't surface the underlying OS error clearly.

**Why it's easy to miss:** The developer always has the binary built locally. The error appears when: another developer clones without building, the binary path changes due to a cargo workspace target directory change, or the plugin is installed from a different location than expected.

**Prevention:**
- Use a build script or `just` task that builds the MCP server binary and copies it to the plugin directory before plugin testing.
- Validate the binary path and permissions in plugin installation documentation.
- Use `command` paths that work after `cargo install` (system PATH) rather than relying on workspace-relative paths for distribution.
- For development, use absolute paths to `target/debug/` or `target/release/` in a local `.mcp.json` override.
- Test the plugin by actually installing it with `claude plugin add` in a clean environment.

**Assay-specific risk:** The plugin is at `plugins/claude-code/` and the MCP server binary will be built by cargo. The `.mcp.json` must reference the binary correctly. During development, the binary is in `target/debug/assay-cli` (or a dedicated MCP server binary); in distribution, it's wherever `cargo install` puts it. These are fundamentally different paths.

---

### P-10: Gate working directory ambiguity

**Area:** Gate Evaluation
**What goes wrong:** `std::process::Command::current_dir()` sets the working directory for the subprocess, but the semantics of "working directory" for a gate are ambiguous. Is it the project root? The spec file's parent directory? The directory the CLI was invoked from? If unspecified, it defaults to the parent process's cwd, which varies depending on how the CLI or MCP server was started.

**Why it's easy to miss:** During local development, cwd is always the project root. Ambiguity surfaces when: the MCP server is started from a different directory, the CLI is invoked from a subdirectory, or specs reference relative paths for gate commands.

**Prevention:**
- Make `working_dir` an explicit required or defaulted field on gate configuration in the spec file.
- Resolve all relative paths against a single, documented anchor (recommend: the directory containing the spec file, or the project root from config).
- In gate evaluation, always set `Command::current_dir()` explicitly; never rely on inherited cwd.
- Log the resolved working directory in GateResult evidence for debuggability.

**Assay-specific risk:** The v0.1 requirements call for "explicit working_dir" on gate evaluation. This is correct. The pitfall is in the resolution logic: if `working_dir` is relative in the TOML spec, relative to what?

---

### P-11: Feature flag misconfiguration for rmcp

**Area:** MCP Server (rmcp)
**What goes wrong:** The `rmcp` crate requires specific feature flags: `server`, `transport-io` (for stdio), and `macros` (for `#[tool_handler]` and `#[tool_router]`). Omitting any of these causes confusing compilation errors about missing traits or methods, not a clear "enable feature X" message.

**Why it's easy to miss:** Cargo features are additive and the rmcp documentation shows snippets without always listing required features. The error messages reference internal trait bounds, not feature flags.

**Prevention:**
- Pin the exact rmcp feature set in `Cargo.toml` workspace dependencies: `rmcp = { version = "...", features = ["server", "transport-io", "macros"] }`.
- Add a comment in `Cargo.toml` explaining why each feature is needed.
- If rmcp requires `tokio`, ensure the workspace tokio dependency has compatible features (rmcp may need specific tokio features beyond `full`).

**Assay-specific risk:** The workspace already has `tokio = { version = "1", features = ["full"] }` in workspace dependencies. Adding rmcp means ensuring its tokio version requirement is compatible. If rmcp pins a different tokio minor version range, cargo will pull two tokio versions or fail to resolve.

---

## Minor Pitfalls

### P-12: `#[serde(default)]` on Option fields in spec TOML creates three-state ambiguity

**Area:** TOML Spec Parsing
**What goes wrong:** For TOML spec fields like `cmd` (optional command for a gate criterion), `Option<String>` with `#[serde(default)]` means: the field is absent = `None`, the field is present with a value = `Some(value)`. But TOML doesn't have null, so there's no way to explicitly represent "this field exists but has no value." If someone writes `cmd = ""`, that's `Some("")`, not `None`. This matters for validation.

**Prevention:**
- Validate that `Some("")` is treated the same as `None` in the trim-then-validate pass, or reject it with a clear error.
- Document in spec file examples that omitting the field is the correct way to indicate "no command."
- Consider using a custom deserializer that trims and converts empty strings to `None`.

---

### P-13: schemars version mismatch between workspace and rmcp

**Area:** Schema Generation / MCP Server
**What goes wrong:** The workspace uses `schemars = "0.8"`. If rmcp depends on a different schemars version (e.g., 0.9 or a fork), two incompatible `JsonSchema` traits exist. Types derived with workspace schemars cannot be used as rmcp tool parameters.

**Prevention:**
- Check rmcp's schemars version requirement before adding it to the workspace.
- If versions differ, create bridge types in the MCP server crate that re-derive `JsonSchema` with rmcp's schemars version.
- Pin schemars in workspace `[workspace.dependencies]` and use `cargo tree -d` to detect duplicate dependency versions.

---

### P-14: Config file search path ordering surprises

**Area:** Config Loading
**What goes wrong:** Config loading typically searches multiple locations (project root, user home, XDG config). If multiple config files exist, the merge order determines which values win. Users expect "closer = higher priority" (project overrides global), but if the implementation loads in the wrong order or doesn't merge (takes the first found), settings disappear.

**Prevention:**
- Define and document a clear, fixed precedence order: project > user > system.
- For v0.1, keep it simple: look for `assay.toml` in the current directory and its ancestors. No merging, no multi-file configs.
- Return an error type that includes which file was loaded, so users can debug "why is it using that config?"

---

### P-15: MCP notification format version mismatch

**Area:** MCP Server (rmcp)
**What goes wrong:** The MCP specification changed the `notifications/message` format between versions. The current spec (2025-06-18) uses `{ level, data, logger? }`. Some clients still expect the old format `{ level, message }`. If the rmcp version implements one format and the Claude Code client expects another, the connection drops immediately after handshake.

**Prevention:**
- Use the latest rmcp release that targets the current MCP specification.
- Test the MCP server against the actual Claude Code client, not just unit tests or a mock client.
- Pin the rmcp version in `Cargo.toml` (not a range) to prevent surprise upgrades that change protocol behavior.
- Check the rmcp changelog for protocol version bumps before upgrading.

---

### P-16: `color-eyre` and MCP server conflict

**Area:** Error Handling / MCP Server
**What goes wrong:** The workspace uses `color-eyre` for error reporting. `color-eyre`'s panic hook and error handler write to stderr with ANSI colors and install a global panic handler. In the CLI, this is desirable. In the MCP server, the panic handler's stderr output is generally fine (stderr is for logging), but `color-eyre`'s global install (`color_eyre::install()`) can only be called once per process. If both the CLI error handling and MCP server initialization try to install it, the second call panics.

**Prevention:**
- Call `color_eyre::install()` only in `main()` of binary crates, never in library code.
- The MCP server binary (or `mcp serve` subcommand) should install its own error handler or share the CLI's.
- `assay-core` must never call `color_eyre::install()`.

---

### P-17: Gate command environment variable leakage

**Area:** Gate Evaluation
**What goes wrong:** `std::process::Command` inherits the parent process's environment by default. If the MCP server or CLI has sensitive environment variables (API keys, tokens), they leak into gate subprocesses. Gate commands authored by users (potentially from spec files shared across teams) run with full access to the parent's environment.

**Prevention:**
- Use `Command::env_clear()` before setting specific environment variables the gate needs.
- Whitelist which environment variables gates can access, or provide an explicit `env` map in the gate configuration.
- At minimum, document that gate commands inherit the parent environment.
- For v0.1, consider a clear default: inherit `PATH`, `HOME`, `USER`, `TMPDIR` only.

---

### P-18: Workspace dependency version for `toml` crate not declared

**Area:** Config Loading / TOML Spec Parsing
**What goes wrong:** The current `Cargo.toml` workspace dependencies do not include the `toml` crate. If individual crates add it independently with different version ranges, cargo may resolve to different versions across the workspace, or worse, the `toml` crate's serde feature interacts differently with the workspace's serde version.

**Prevention:**
- Add `toml = { version = "0.8", features = ["parse"] }` to `[workspace.dependencies]` before any crate imports it.
- Ensure the `toml` version's serde dependency is compatible with the workspace's `serde = "1"`.
- Use `cargo deny` (already configured) to flag duplicate crate versions.

---

## Integration Pitfalls (Cross-Cutting)

### P-19: assay-types DTOs used as MCP parameters need dual-derive compatibility

**Area:** Types Crate + MCP Server
**What goes wrong:** `assay-types` already derives `Serialize`, `Deserialize`, and `JsonSchema` on all DTOs. If these types are reused as rmcp tool parameters, they must also satisfy rmcp's `Parameters<T>` constraint, which requires `DeserializeOwned + JsonSchema`. This works, but adding `#[serde(deny_unknown_fields)]` to types for TOML strictness breaks MCP parameter deserialization if the MCP client sends extra metadata fields.

**Prevention:**
- Separate TOML-facing types (strict, with `deny_unknown_fields`) from MCP-facing parameter types (permissive).
- Alternatively, use wrapper types in the MCP server crate: `struct SpecGetParams { name: String }` rather than reusing `assay_types::Spec` directly.
- Test both TOML deserialization and MCP parameter deserialization for any shared type.

---

### P-20: MCP server `mcp serve` subcommand vs. standalone binary decision

**Area:** CLI + MCP Server
**What goes wrong:** If the MCP server runs as a CLI subcommand (`assay mcp serve`), the CLI's initialization code (clap parsing, color-eyre install, config loading) runs before the MCP server starts. The CLI might print version info, help text, or errors to stdout before the MCP transport takes over, corrupting the protocol. If it's a standalone binary, the workspace layout and build pipeline need to support two binary targets.

**Prevention:**
- If using a subcommand approach: ensure clap does not print to stdout on parse errors (configure clap to use stderr via `Command::color(ColorChoice::Auto)` and custom error handling).
- The `mcp serve` subcommand must suppress all non-protocol stdout output immediately.
- Test by running `assay mcp serve` and verifying the first byte on stdout is `{` (start of a JSON-RPC message).
- Alternatively, create a separate `assay-mcp` binary crate with minimal initialization.

**Assay-specific risk:** The v0.1 requirements list `mcp serve` as a CLI subcommand. This means the CLI crate depends on the MCP server functionality. The CLI's stdout management must be airtight when this subcommand is invoked.

---

## Summary

| ID | Severity | Area | Core Issue |
|---|---|---|---|
| P-01 | Critical | MCP/stdio | Stdout corruption breaks protocol |
| P-02 | Critical | Gate eval | Pipe buffer deadlock on large output |
| P-03 | Critical | MCP+Gate | Blocking sync code in async runtime |
| P-04 | Critical | Gate eval | No timeout, zombie/orphan processes |
| P-05 | Moderate | TOML | External enum tagging unsupported |
| P-06 | Moderate | TOML/Config | Poor error messages for users |
| P-07 | Moderate | Error types | Implicit `#[from]` hides context |
| P-08 | Moderate | MCP/types | schemars/serde schema divergence |
| P-09 | Moderate | Plugin | Binary path resolution failures |
| P-10 | Moderate | Gate eval | Ambiguous working directory |
| P-11 | Moderate | MCP/rmcp | Feature flag misconfiguration |
| P-12 | Minor | TOML | Empty string vs absent field |
| P-13 | Minor | Schema/MCP | schemars version conflict |
| P-14 | Minor | Config | Search path ordering |
| P-15 | Minor | MCP | Protocol version mismatch |
| P-16 | Minor | Error/MCP | color-eyre global install conflict |
| P-17 | Minor | Gate eval | Environment variable leakage |
| P-18 | Minor | Config/TOML | Workspace dep not declared |
| P-19 | Integration | Types+MCP | Dual-derive compatibility |
| P-20 | Integration | CLI+MCP | Subcommand stdout corruption |

---

## Sources

- [Shuttle: How to Build a stdio MCP Server in Rust](https://www.shuttle.dev/blog/2025/07/18/how-to-build-a-stdio-mcp-server-in-rust)
- [Write your MCP servers in Rust](https://rup12.net/posts/write-your-mcps-in-rust/)
- [rmcp docs.rs](https://docs.rs/rmcp/latest/rmcp/)
- [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk)
- [MCP Transports specification](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports)
- [Codex rmcp stdio compatibility issue](https://github.com/openai/codex/issues/5671)
- [Rust std::process::Command stdout deadlock (Issue #45572)](https://github.com/rust-lang/rust/issues/45572)
- [Rust std::process::Child documentation](https://doc.rust-lang.org/std/process/struct.Child.html)
- [Tokio: Bridging with sync code](https://tokio.rs/tokio/topics/bridging)
- [Claude Code MCP documentation](https://code.claude.com/docs/en/mcp)
- [SFEIR MCP Troubleshooting](https://institute.sfeir.com/en/claude-code/claude-code-mcp-model-context-protocol/troubleshooting/)
- [Serde enum representations](https://serde.rs/enum-representations.html)
- [TOML externally tagged enum issue](https://github.com/alexcrichton/toml-rs/issues/225)
- [Error type design in Rust](https://nrc.github.io/error-docs/error-design/error-type-design.html)
- [Designing error types in Rust libraries](https://d34dl0ck.me/rust-bites-designing-error-types-in-rust-libraries/index.html)
- [Serde + TOML + deserialize_with on Options](https://users.rust-lang.org/t/serde-toml-deserialize-with-on-options/77347)
- [thiserror docs.rs](https://docs.rs/thiserror/latest/thiserror/)
- [schemars attributes documentation](https://graham.cool/schemars/deriving/attributes/)

---

*Research completed: 2026-02-28*
