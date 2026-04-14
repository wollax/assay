---
title: auto_promote_on_pass has TOCTOU window between two gate file reads
area: assay-core
severity: nitpick
source: PR #7 code review
---

`auto_promote_on_pass` reads the gates file to check status, then `spec_set_status` reads it again before writing. Between the two loads there's a TOCTOU window. Low practical impact in single-agent workflow but inconsistent with the atomic-write pattern used elsewhere. Consider combining into a single load-mutate-save.

File: `crates/assay-core/src/spec/mod.rs`
