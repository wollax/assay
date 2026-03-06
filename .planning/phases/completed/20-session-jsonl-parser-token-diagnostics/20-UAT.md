# Phase 20 UAT: Session JSONL Parser & Token Diagnostics

**Date:** 2026-03-06
**Result:** 10/10 PASS

| # | Test | Status |
|---|------|--------|
| 1 | `assay context list` displays sessions table | PASS |
| 2 | `assay context list --tokens` adds token column | PASS |
| 3 | `assay context list --json` outputs valid JSON | PASS |
| 4 | `assay context list --plain` has no ANSI codes | PASS |
| 5 | `assay context diagnose` displays dashboard | PASS |
| 6 | `assay context diagnose --json` outputs valid JSON | PASS |
| 7 | `assay context diagnose --plain` has no ANSI codes | PASS |
| 8 | `assay context diagnose` shows health indicator | PASS |
| 9 | `assay context list --all` shows all sessions | PASS |
| 10 | `just ready` passes clean | PASS |
