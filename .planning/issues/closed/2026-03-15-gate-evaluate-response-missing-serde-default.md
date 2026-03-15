# GateEvaluateResponse.diff_truncation missing serde(default)

**Source:** PR review (Phase 44)
**Severity:** Suggestion
**File:** crates/assay-mcp/src/server.rs

`GateEvaluateResponse.diff_truncation` uses `#[serde(skip_serializing_if)]` without `#[serde(default)]`. The struct is currently Serialize-only so this is benign, but if `Deserialize` is ever added, the missing `default` will cause deserialization failures for records that omit the field.
