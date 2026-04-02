# S02: README + example manifest documentation — UAT

**Milestone:** M009
**Written:** 2026-03-24

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: This slice produces only documentation files (README.md and example TOML comments). No runtime behavior changed. Verification is reading the artifacts and confirming they parse correctly.

## Preconditions

- Smelt repo checked out at the S02 branch or main after merge
- `cargo build` succeeds (for `--dry-run` verification)

## Smoke Test

Open `README.md` at the workspace root. Confirm it has a title, install section, and at least 6 subcommand sections (init, list, run, serve, status, watch).

## Test Cases

### 1. README completeness

1. Open `README.md`
2. Check for these sections: Install, Quickstart, `smelt init`, `smelt list`, `smelt run`, `smelt serve`, `smelt status`, `smelt watch`, Examples, Ecosystem
3. **Expected:** All sections present with usage blocks and flag descriptions

### 2. README accuracy — flags match --help

1. Run `cargo run -- run --help`
2. Compare the flags listed in the README's `smelt run` section against the actual output
3. Repeat for `cargo run -- serve --help` and the README's `smelt serve` section
4. **Expected:** Every flag name, short flag, and description in the README matches `--help` output exactly

### 3. Example manifests parse correctly

1. Run `cargo run -- run examples/job-manifest.toml --dry-run`
2. Run `cargo run -- run examples/agent-manifest.toml --dry-run`
3. Run `cargo run -- run examples/job-manifest-compose.toml --dry-run`
4. Run `cargo run -- run examples/job-manifest-forge.toml --dry-run`
5. Run `cargo run -- run examples/job-manifest-k8s.toml --dry-run`
6. **Expected:** All 5 commands exit 0 with a valid execution plan

### 4. Bad manifest still fails correctly

1. Run `cargo run -- run examples/bad-manifest.toml --dry-run`
2. **Expected:** Exit non-zero with validation errors for: empty job.name, empty image, zero timeout, duplicate session name, unknown depends_on, empty merge.target, unknown merge.order

### 5. Example comments are present and useful

1. Open `examples/job-manifest-k8s.toml`
2. Check that every field (`namespace`, `service_account`, `cpu_request`, etc.) has an inline `#` comment
3. Open `examples/bad-manifest.toml`
4. Check that each intentional error has a `# VIOLATION:` comment explaining what rule it breaks
5. **Expected:** Comments are present, accurate, and helpful for a new user

## Edge Cases

### agent-manifest.toml is a real valid manifest

1. Run `cargo run -- run examples/agent-manifest.toml --dry-run`
2. Check the execution plan shows sessions, merge config, and environment
3. **Expected:** Exit 0 — this file was previously broken and should now be a complete minimal example

## Failure Signals

- Any `--dry-run` command exits non-zero for a valid example file
- README references a flag that doesn't exist in `--help` output
- README is missing a subcommand section
- Example file has fields with no comments

## Requirements Proved By This UAT

- R041 — README exists with project overview, install, all 6 subcommands, and examples. Human confirms readability and accuracy.
- R045 — All 7 example manifests have inline field-level comments. Human confirms comments are clear and useful for a new user.

## Not Proven By This UAT

- README long-term maintenance (staying in sync with future CLI changes is not tested)
- Automated README accuracy regression testing (no CI check that README matches --help)

## Notes for Tester

- The README is the primary user entry point — read it as if you've never seen Smelt before and note anything confusing
- Focus on whether the quickstart section would actually get a new user from zero to a working `--dry-run` invocation
- The `agent-manifest.toml` was completely rewritten (it was broken before) — verify it feels like a good minimal example
