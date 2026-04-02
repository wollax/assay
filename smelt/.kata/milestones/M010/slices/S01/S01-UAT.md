# S01: Bearer token auth middleware with read/write split — UAT

**Milestone:** M010
**Written:** 2026-03-24

## UAT Type

- UAT mode: live-runtime
- Why this mode is sufficient: Auth behavior is HTTP request/response — best verified by sending real curl requests against a running `smelt serve` instance with auth configured

## Preconditions

- `smelt` binary built and on PATH
- A `server.toml` with `[auth]` section configured:
  ```toml
  [auth]
  write_token_env = "SMELT_WRITE_TOKEN"
  read_token_env = "SMELT_READ_TOKEN"
  ```
- Environment variables set:
  ```bash
  export SMELT_WRITE_TOKEN="write-secret-123"
  export SMELT_READ_TOKEN="read-secret-456"
  ```
- `smelt serve --config server.toml` running (default port 3000)

## Smoke Test

```bash
curl -s http://localhost:3000/api/v1/jobs
# Expected: 401 with JSON error body (auth is enabled, no token provided)
```

## Test Cases

### 1. No auth header returns 401

1. `curl -s -w '\n%{http_code}' http://localhost:3000/api/v1/jobs`
2. **Expected:** HTTP 401, JSON body containing `"error"` mentioning "Authorization"

### 2. Invalid token returns 403

1. `curl -s -w '\n%{http_code}' -H 'Authorization: Bearer wrong-token' http://localhost:3000/api/v1/jobs`
2. **Expected:** HTTP 403, JSON body containing `"error"` mentioning "permission"

### 3. Read token allows GET

1. `curl -s -w '\n%{http_code}' -H 'Authorization: Bearer read-secret-456' http://localhost:3000/api/v1/jobs`
2. **Expected:** HTTP 200, JSON array of jobs (empty `[]` is fine)

### 4. Read token blocked on POST

1. `curl -s -w '\n%{http_code}' -X POST -H 'Authorization: Bearer read-secret-456' -H 'Content-Type: application/toml' -d '' http://localhost:3000/api/v1/jobs`
2. **Expected:** HTTP 403, JSON body mentioning "write permission"

### 5. Write token allows GET and POST

1. `curl -s -w '\n%{http_code}' -H 'Authorization: Bearer write-secret-123' http://localhost:3000/api/v1/jobs`
2. **Expected:** HTTP 200
3. `curl -s -w '\n%{http_code}' -X POST -H 'Authorization: Bearer write-secret-123' -H 'Content-Type: application/toml' -d '[job]\nname = "test"' http://localhost:3000/api/v1/jobs`
4. **Expected:** HTTP 200 or 400 (bad manifest is fine — auth passed, the error is about content not auth)

### 6. No auth section = no auth (backward compat)

1. Remove `[auth]` section from `server.toml`, restart `smelt serve`
2. `curl -s -w '\n%{http_code}' http://localhost:3000/api/v1/jobs`
3. **Expected:** HTTP 200 (no auth enforced)

## Edge Cases

### Missing env var at startup

1. Set `write_token_env = "NONEXISTENT_VAR"` in `[auth]`, unset the env var
2. Run `smelt serve --config server.toml`
3. **Expected:** Startup fails with error message naming `NONEXISTENT_VAR`

### Malformed Authorization header

1. `curl -s -w '\n%{http_code}' -H 'Authorization: Basic dXNlcjpwYXNz' http://localhost:3000/api/v1/jobs`
2. **Expected:** HTTP 401 (not Bearer format)

## Failure Signals

- Any request returning 200 when it should return 401 or 403
- Server starting successfully with missing/empty auth env vars
- Error responses missing JSON body or descriptive error messages
- Read token being accepted for POST/DELETE requests

## Requirements Proved By This UAT

- R050 — Bearer token auth works end-to-end with real HTTP requests against a running server; 401 on missing header; startup fails on missing env var
- R051 — Read/write split verified: read token GET→200, POST→403; write token has full access; backward compat with no `[auth]` section

## Not Proven By This UAT

- Token comparison timing safety (constant-time comparison) — would require specialized timing analysis
- Behavior under high concurrency or load — not tested
- Integration with TLS/reverse proxy (production deployment patterns)

## Notes for Tester

- The POST request in test case 5 will likely return 400 (bad manifest) — that's expected. The point is that auth passes (not 401/403).
- Check the server logs for `tracing::warn` on rejected requests — should show method + path but never token values.
- Check startup logs for `tracing::info` showing which auth env var names are configured.
