---
title: Document on_event display string change in CLI output
area: assay-cli
severity: suggestion
source: PR review (Phase 61)
---

`assay spec review` now prints `on_event(...)` where it previously printed `at_event(...)`. This is a user-visible output change. If users or scripts match this string literally, it could silently break. Consider documenting in changelog.
