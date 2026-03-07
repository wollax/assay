---
area: testing
severity: low
source: phase-12 PR review
---

# Missing evaluate_all test for FileExists failure path

`evaluate_all_includes_file_exists_criteria` only covers the passing case. Add a test where a path-only criterion points to a missing file and is counted as `failed`.
