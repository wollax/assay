# Phase 16: Agent Gate Recording — UAT

## Tests

| # | Test | Status |
|---|------|--------|
| 1 | Agent-evaluated spec parses with kind=AgentReport criteria | PASS |
| 2 | gate_run skips AgentReport criteria (marks as pending/skipped) | PASS |
| 3 | Spec validation rejects kind=AgentReport combined with cmd/path | PASS |
| 4 | CLI output shows [cmd]/[file]/[agent] kind labels | PASS |
| 5 | Schema snapshots include all new types | PASS |
| 6 | just ready passes | PASS |

## Results

**6/6 tests passed**

### Test Details

1. **Spec parsing** — Created spec with `kind = "AgentReport"` + `prompt` + `enforcement = "advisory"`. `assay spec list` shows it correctly.

2. **Gate run** — Mixed spec (cmd + agent): command criterion runs and shows `[cmd] unit-tests ... ok`, agent criterion shows `[agent] code-review ... pending`. Summary: 1 passed, 1 skipped.

3. **Validation** — Spec with both `kind = "AgentReport"` and `cmd` rejected: "criterion has kind=AgentReport with `cmd`; agent criteria cannot have a command"

4. **Kind labels** — All three label types confirmed in single run: `[cmd] tests`, `[file] readme`, `[agent] review`. Colors render correctly.

5. **Schema snapshots** — 5 new snap files: criterion-kind, evaluator-role, confidence, agent-evaluation, agent-session.

6. **Quality gate** — `just ready`: fmt-check, clippy (-D warnings), 260 tests (3 ignored), cargo-deny all pass.
