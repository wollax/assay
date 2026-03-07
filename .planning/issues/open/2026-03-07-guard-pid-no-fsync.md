---
created: 2026-03-07T08:00
title: create_pid_file doesn't fsync — crash could leave partial PID
area: assay-core
severity: important
files:
  - crates/assay-core/src/guard/pid.rs:58
---

## Problem

`create_pid_file` uses `fs::write` which does not guarantee the data is flushed to disk. A crash or power loss between the write and the OS flush could leave a partial or empty PID file, which `check_running` would then treat as corrupt and silently remove, potentially allowing a second daemon instance to start.

## Solution

Use `File::create` + `write_all` + `sync_all` (or `sync_data`) to ensure the PID is durably written before the function returns.
