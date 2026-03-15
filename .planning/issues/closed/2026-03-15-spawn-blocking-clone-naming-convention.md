# `spawn_blocking` clone naming convention should use idiomatic shadowing

**Area:** crates/assay-mcp/src/server.rs
**Severity:** Low
**Source:** PR #43 review (phase-43-gate-evaluate-schema)

## Description

Variables cloned for use inside `spawn_blocking` closures use ad-hoc suffixes (`_clone`, `_for_save`, `_owned`), which is inconsistent and noisy. The idiomatic Rust pattern is to shadow the binding in a new scope immediately before the `move` closure, keeping the original name.

## Suggested Fix

Use a shadowing block before the closure:

```rust
let config = config.clone();
let session_id = session_id.clone();
spawn_blocking(move || {
    // config and session_id are the cloned copies
})
```

This avoids inventing new names and is the pattern used by Tokio's own documentation.

## Category

style
