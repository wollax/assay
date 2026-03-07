# self-check.toml: cargo fmt missing --all flag

**Source:** PR review (Phase 19)
**Area:** .assay/specs/self-check.toml
**Priority:** low

The `formatting` criterion uses `cargo fmt --check` without `--all`, while linting and tests use `--workspace`. Should be `cargo fmt --check --all` for consistency.
