# `save_session` JSON Error Points to Directory, Not File; Operation String Missing Session ID

## Description

The JSON serialization error path in `save_session` passes `&sessions_dir` as the path context to `AssayError::json`:

```rust
.map_err(|e| AssayError::json("serializing work session", &sessions_dir, e))?;
```

At this point the final file path (`sessions_dir.join(format!("{}.json", session.id))`) is known. The error should reference that path so the caller can identify exactly which file failed. Additionally, the operation string `"serializing work session"` does not include the session ID, making it harder to correlate with logs when many sessions exist concurrently.

## File Reference

`crates/assay-core/src/work_session.rs` — `save_session`, line 89

## Category

error-handling
