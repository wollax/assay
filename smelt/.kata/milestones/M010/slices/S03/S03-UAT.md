# S03: Documentation and final verification — UAT

**Milestone:** M010
**Written:** 2026-03-24

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: This slice only adds documentation (examples/server.toml comments, README.md text). No runtime behavior to test — only human readability of the written docs.

## Preconditions

- Repository checked out at the S03 branch or main after merge
- `examples/server.toml` and `README.md` present

## Smoke Test

Open `README.md` and search for "Authentication" — a subsection should exist under Server Mode explaining bearer token auth configuration.

## Test Cases

### 1. server.toml auth section is clear and complete

1. Open `examples/server.toml`
2. Find the `[auth]` section (commented out)
3. Read the inline comments
4. **Expected:** `write_token_env` and `read_token_env` fields are documented with clear explanations that these are env var names (not raw tokens), that `write_token_env` is required when auth is enabled, and that `read_token_env` is optional for read-only access

### 2. README Authentication subsection is accurate

1. Open `README.md`
2. Navigate to the Server Mode section
3. Find the "Authentication" subsection
4. **Expected:** Documents opt-in behavior (no `[auth]` = open access), env var config pattern with TOML example, read/write permission model (GET/HEAD = read, others = write), and 401 vs 403 error distinction

### 3. Examples table mentions auth

1. In `README.md`, find the examples directory table
2. Find the `server.toml` row
3. **Expected:** Description mentions auth configuration

## Edge Cases

### Readability of commented-out TOML

1. Open `examples/server.toml` in a plain text editor
2. Verify the `[auth]` section follows the same comment style as other sections
3. **Expected:** Comments are consistent with the rest of the file; uncommenting the section would produce valid TOML

## Failure Signals

- `[auth]` section missing from server.toml
- Authentication subsection missing from README
- Documentation describes raw tokens instead of env var names
- Documentation contradicts actual auth behavior (e.g., wrong HTTP methods for read/write split)

## Requirements Proved By This UAT

- R050 — Human verifies the documentation is accurate and complete for bearer token auth config
- R051 — Human verifies the read/write permission model documentation is clear and correct

## Not Proven By This UAT

- Actual auth middleware behavior (proven by automated tests in S01)
- Teardown error handling improvements (R052, proven by automated tests in S02)
- SSH DRY cleanup (R053, proven by automated tests in S02)

## Notes for Tester

This is a documentation-only UAT. The goal is readability and accuracy — does the documentation match what S01 actually implemented? If anything seems unclear or contradicts expected behavior, flag it.
