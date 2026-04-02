# S03: Documentation and final verification — Research

**Date:** 2026-03-24

## Summary

S03 is a documentation-only slice. The two deliverables are: (1) add an `[auth]` section to `examples/server.toml` with inline comments documenting the config format, and (2) update the README.md Server Mode section to describe authentication configuration. Both S01 and S02 are complete — no code changes needed, only docs and a final milestone verification pass.

The work is straightforward. The `[auth]` config format is already implemented in `config.rs` (`AuthConfig` struct with `write_token_env: String` and `read_token_env: Option<String>`). The README already has a Server Mode section at line 222 with a TOML code block and subsections for HTTP API, queue persistence, SSH workers, and TUI. Auth documentation slots in naturally as a new subsection.

## Recommendation

Single task is sufficient — add `[auth]` to `examples/server.toml`, add an "Authentication" subsection to README.md Server Mode, update the server.toml description in the examples table to mention auth, then run the full verification pass (`cargo test/clippy/doc`).

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Auth config format reference | `crates/smelt-cli/src/serve/config.rs:74-86` — `AuthConfig` struct | Authoritative source for field names and types |
| README structure | Existing Server Mode section (lines 222-275) | Follow the established subsection pattern |
| Example comment style | `examples/server.toml` existing pattern | All fields have `#` inline comments; follow same style |

## Existing Code and Patterns

- `examples/server.toml` — 67 lines, uses `# comments` above each field with explanatory text. The `[server]` and `[[workers]]` sections are commented examples. `[auth]` should follow the same pattern: commented-out by default with clear field docs.
- `crates/smelt-cli/src/serve/config.rs:74-86` — `AuthConfig { write_token_env: String, read_token_env: Option<String> }` with `deny_unknown_fields`. Two fields only.
- `crates/smelt-cli/src/serve/http_api.rs:69-97` — `ResolvedAuth` and `resolve_auth()` — documents the env var resolution behavior (fails fast on missing/empty).
- `crates/smelt-cli/src/serve/http_api.rs:111-170` — `auth_middleware()` — documents the permission model (GET/HEAD = read, else = write).
- `README.md:222-275` — Server Mode section with TOML code block, 4 subsections (Job Submission, HTTP API, Queue Persistence, SSH Worker Pools, Live TUI). Auth subsection goes between "HTTP API Endpoints" and "Queue Persistence".

## Constraints

- `server.toml` uses `deny_unknown_fields` — the `[auth]` section in the example must exactly match `AuthConfig` fields (`write_token_env`, `read_token_env`)
- D014/D112 pattern: document that config contains env var **names**, never raw token values
- D134: auth is opt-in — example should show it commented-out by default
- D136: `write_token_env` is required when `[auth]` is present; `read_token_env` is optional
- D137: GET/HEAD = read permission, all other methods = write permission

## Common Pitfalls

- **Writing raw token values in examples** — Must show env var names (e.g. `SMELT_WRITE_TOKEN`), never token values. Add a clear comment about this.
- **Forgetting to mention 401 vs 403 distinction** — README should note: missing/malformed header → 401, valid format but wrong token → 403, insufficient permission → 403.
- **Not mentioning backward compat** — README must state that omitting `[auth]` means no authentication (current behavior preserved).

## Open Risks

- None. This is pure documentation with no code changes. The verification pass confirms all milestone success criteria are already met by S01 and S02.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| N/A | N/A | Documentation-only slice — no technology skills needed |

## Sources

- `AuthConfig` struct in `config.rs` — field names and types
- S01 summary — auth middleware behavior, test coverage, error messages
- S02 summary — teardown and SSH cleanup confirmation
- Existing `examples/server.toml` — comment style convention
- Existing `README.md` Server Mode section — structure convention
