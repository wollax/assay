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
