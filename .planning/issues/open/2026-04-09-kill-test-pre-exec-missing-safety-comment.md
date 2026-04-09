---
title: Missing SAFETY comment on unsafe pre_exec in kill_helper test
severity: low
area: assay-core/pipeline_checkpoint
source: PR review (Phase 60)
---

The `kill_helper_terminates_long_running_process` test uses `unsafe pre_exec(|| { libc::setpgid(0, 0); Ok(()) })`. While `setpgid` is async-signal-safe and this is correct, the `unsafe` block lacks a SAFETY comment explaining why post-fork usage is sound. Add one to prevent future contributors from adding allocating calls in the `pre_exec` closure.
