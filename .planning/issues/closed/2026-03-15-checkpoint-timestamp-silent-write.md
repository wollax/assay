# Checkpoint Timestamp File Write Silently Ignored

## Description

In `save_checkpoint`, the write to `.last-checkpoint-ts` uses `let _ = std::fs::write(...)`, which silently discards any I/O error:

```rust
let _ = std::fs::write(&ts_path, now.to_string());
```

`save_checkpoint` already returns a `Result` and uses `atomic_write` (which propagates errors) for the two content files. The timestamp file write should be treated consistently: either use `atomic_write` / propagate the error, or at minimum log a warning rather than discarding it entirely. Silent failure here makes the `.last-checkpoint-ts` file unreliable as a sentinel for guard daemon freshness checks.

## File Reference

`crates/assay-core/src/checkpoint/persistence.rs` — `save_checkpoint`, line 68

## Category

error-handling / reliability
