# `stale_threshold` Accepts Zero, Which Would Abandon All Running Sessions on Startup

## Description

`stale_threshold: u64` has no minimum-value validation. A value of `0` means every `AgentRunning` session is immediately considered stale the moment the server starts, which would abandon all active sessions on every restart. A validation guard (at config-load time or in `SessionsConfig::new`) should reject zero and emit a clear error message.

## File Reference

`crates/assay-types/src/lib.rs` — `SessionsConfig.stale_threshold`

## Category

validation / correctness
