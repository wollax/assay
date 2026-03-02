# Phase 5: Config and Initialization — UAT

**Date:** 2026-03-02
**Status:** Passed (6/6)

## Tests

| # | Test | Expected | Result |
|---|------|----------|--------|
| 1 | `assay init` in fresh directory | Creates `.assay/config.toml`, `.assay/specs/hello-world.toml`, `.assay/.gitignore`, `.assay/specs/` | Pass |
| 2 | `assay init` second run | Fails with ".assay/ already exists" error, exit code 1 | Pass |
| 3 | `assay init --name custom` | config.toml contains `project_name = "custom-project"` | Pass |
| 4 | config.toml has commented-out gates | Contains `# [gates]`, `# default_timeout = 300` | Pass |
| 5 | hello-world.toml has both criteria modes | One criterion with `cmd =`, one without | Pass |
| 6 | Config with unknown key rejected | `from_str` rejects TOML with unknown fields | Pass |
