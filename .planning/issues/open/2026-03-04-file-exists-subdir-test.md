---
area: testing
severity: low
source: phase-12 PR review
---

# Missing test for FileExists with subdirectory paths

No test verifies that `path` containing a subdirectory (e.g., `"subdir/file.txt"`) resolves correctly relative to working_dir.
