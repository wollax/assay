---
title: Inline checkpoint-abort killpg has no SIGKILL fallback
severity: low
area: assay-core/pipeline
source: PR review (Phase 60)
---

The inline checkpoint-abort path in `pipeline.rs:944-961` only sends SIGTERM via `killpg` and immediately drains the channel — no grace period, no SIGKILL escalation. If the agent subprocess ignores SIGTERM, it remains alive after the pipeline returns.

`kill_agent_subprocess` in `pipeline_checkpoint.rs` already implements the full SIGTERM → poll → SIGKILL escalation. Consider delegating to it instead of inlining a partial kill.
