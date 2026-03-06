# Phase 18 UAT: CLI Hardening & Enforcement Surface

## Results: 10/10 passed

| # | Test | Status |
|---|------|--------|
| 1 | Single process::exit in main() only | PASS |
| 2 | No .join(".assay") string literals remain | PASS |
| 3 | Gate help says "Manage quality gates" | PASS |
| 4 | Bare `assay` inside project shows status, exits 0 | PASS |
| 5 | Bare `assay` outside project prints help, exits non-zero | PASS |
| 6 | Error propagation shows cause chain | PASS |
| 7 | Advisory failure displays yellow WARN with [advisory] prefix | PASS |
| 8 | Required failure displays red FAILED (unchanged) | PASS |
| 9 | Summary line includes warned category | PASS |
| 10 | Advisory-only failures exit 0, required failures exit 1 | PASS |

## Test Evidence

### Test 4: Inside project
```
assay 0.1.0 -- test-project
No specs found in specs
EXIT: 0
```

### Test 5: Outside project
```
Not an Assay project. Run `assay init` to get started.
[help output]
EXIT: 1
```

### Test 6: Error cause chain
```
Error: parsing config `.assay/config.toml`: TOML parse error at line 1...
EXIT: 1
```

### Tests 7-10: Enforcement-aware output
```
  [cmd] required-pass ... ok
  [cmd] [advisory] advisory-fail ... WARN
  [cmd] [advisory] advisory-pass ... ok
Results: 2 passed, 0 failed, 1 warned, 0 skipped (of 3 total)
EXIT: 0
```

Required failure case:
```
  [cmd] required-fail ... FAILED
  [cmd] [advisory] advisory-fail ... WARN
Results: 0 passed, 1 failed, 1 warned, 0 skipped (of 2 total)
EXIT: 1
```
