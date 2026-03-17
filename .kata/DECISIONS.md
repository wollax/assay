# Decisions Register

<!-- Append-only. Never edit or remove existing rows.
     To reverse a decision, add a new row that supersedes it.
     Read this file at the start of any planning or research phase. -->

| #    | When     | Scope   | Decision                       | Choice                                          | Rationale                                                        | Revisable?                  |
| ---- | -------- | ------- | ------------------------------ | ----------------------------------------------- | ---------------------------------------------------------------- | --------------------------- |
| D001 | M001     | arch    | Smelt role                     | Pure infrastructure layer                        | Assay owns orchestration; Smelt provisions environments          | No                          |
| D002 | M001     | arch    | Assay integration boundary     | Shell out to `assay` CLI, no crate dependency    | Process boundary keeps repos independent                         | No                          |
| D003 | M001     | arch    | v0.1.0 code disposition        | Gut entirely, start fresh                        | Orchestration logic migrates to Assay; manifest schema is new    | No                          |
| D004 | M001     | arch    | Runtime abstraction            | Pluggable RuntimeProvider trait                  | Docker first, Compose/K8s later via same trait                   | No                          |
| D005 | M001     | library | Docker client                  | bollard crate                                    | De facto Rust Docker client, async/tokio native                  | Yes — if exec API unreliable |
| D010 | M001     | arch    | Manifest authorship            | Assay generates manifests, Smelt consumes        | Single source of truth for job specs lives in Assay              | No                          |
| D012 | M001     | scope   | Image building                 | Pre-built images only, no Dockerfile building    | Simplifies M001 scope; users provide images                      | Yes — if user demand        |
| D013 | M001     | arch    | Repo delivery to container     | Bind-mount host repo into container              | Avoids clone overhead; container reads/writes directly to mount  | Yes — if K8s needs volumes  |
| D014 | M001     | arch    | Credential injection           | Environment variable passthrough                 | Simplest secure path; vault integration deferred                 | Yes — if vault needed       |
| D015 | M001     | arch    | Git module reuse               | Keep git/cli.rs and git/mod.rs from v0.1.0       | Branch ops, commit, push are reusable for result collection      | No                          |
| D016 | M001     | scope   | Workspace structure            | Keep smelt-cli + smelt-core two-crate workspace  | Established pattern, no reason to change                         | No                          |
| D017 | M001-S01 | pattern | Manifest strict parsing        | deny_unknown_fields on all 6 manifest structs    | Catches schema mismatches at parse time instead of at runtime    | No                          |
| D018 | M001-S01 | pattern | Validation error aggregation   | Collect all errors before returning, not fail-fast | Users see every issue at once instead of fixing one at a time   | No                          |
| D019 | M001-S01 | library | Async trait style              | RPITIT instead of async_trait macro               | Rust 2024 edition supports this natively; avoids boxing overhead | Yes — if edition downgraded |
| D020 | M001-S01 | pattern | Config missing file behavior   | Return defaults when .smelt/config.toml missing   | Non-fatal — first run should work without config file            | No                          |
| D021 | M001-S02 | pattern | Container keep-alive strategy  | `sleep 3600` as container CMD, work via exec      | Container stays running while exec commands are issued against it | No                          |
| D022 | M001-S02 | pattern | Container labeling             | `smelt.job=<name>` label on all containers        | Enables identification and cleanup via `docker ps --filter`       | No                          |
| D023 | M001-S02 | pattern | Teardown guarantee             | Explicit teardown in both success and error paths  | No scopeguard/Drop — explicit match ensures cleanup visibility    | Yes — if signal handling in S05 changes pattern |
| D024 | M001-S02 | pattern | Docker test skip pattern       | Tests skip gracefully when daemon unavailable       | Keeps `cargo test --workspace` green in all environments          | No                          |
| D025 | M001-S02 | pattern | ExecHandle result fields       | exit_code/stdout/stderr on ExecHandle directly      | Simpler API — results returned to caller without indirection      | No                          |
| D026 | M001-S02 | pattern | CLI teardown via async block   | Async block for exec work, teardown unconditional   | Guarantees cleanup without Drop/scopeguard complexity             | Yes — if signal handling in S05 changes pattern |
| D027 | M001-S03 | pattern | Container workspace path       | Fixed `/workspace` mount point for host repo         | Simple convention; all exec commands use `working_dir: /workspace` | Yes — if multi-repo support needed |
| D028 | M001-S03 | pattern | Assay manifest delivery        | Base64-encode TOML, write via exec `base64 -d`       | Avoids heredoc quoting issues with TOML special chars             | Yes — if Docker put_archive proves simpler |
| D029 | M001-S03 | pattern | Assay manifest translation     | Smelt-side serde structs for Assay format, no import  | Keeps D002 (no crate dep); mock-based testing for now             | Yes — when real Assay integration in S06 |
| D030 | M001-S03 | scope   | Repo path validation           | Local paths only; URLs rejected with clear error      | Bind-mount requires local path; clone support deferred            | Yes — if clone-into-container added later |
| D031 | M001-S04 | pattern | ResultCollector generic design  | Generic `<G: GitOps>` not `dyn GitOps`                | RPITIT (D019) makes trait not object-safe; generics keep testability | No |
| D032 | M001-S04 | pattern | Host-side collection            | Collector reads host repo directly, not via Docker exec | Bind-mount (D013) means commits are already on host filesystem     | No |
| D033 | M001-S04 | pattern | Target branch force-update      | Delete + recreate target branch if it exists            | Simple and explicit; warns with old/new hashes for auditability    | Yes — if append-mode needed |
| D034 | M001-S05 | pattern | Monitor state file format        | TOML at `.smelt/run-state.toml`, single-job model       | TOML already in deps; no new dependency; sufficient for M001 single-job | Yes — if concurrent jobs needed |
| D035 | M001-S05 | pattern | Timeout source                   | Max session timeout from manifest, fallback to config default | Matches existing `AssayInvoker::build_run_command()` logic; no new manifest field | Yes — if explicit job-level timeout field added |
| D036 | M001-S05 | pattern | Signal handling via tokio::select! | `ctrl_c()` + `sleep(timeout)` racing against exec future | Standard tokio pattern; testable via cancellation abstraction | No |
