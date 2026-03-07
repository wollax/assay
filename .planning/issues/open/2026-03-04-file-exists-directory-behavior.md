---
area: design
severity: low
source: phase-12 PR review
---

# FileExists behavior for directories is undocumented

`Path::exists()` returns `true` for directories. A criterion pointing at a directory passes silently. Should this be intentional? Document or restrict.
