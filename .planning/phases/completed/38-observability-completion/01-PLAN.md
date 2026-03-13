---
phase: 38
plan: 1
wave: 1
depends_on: []
files_modified:
  - crates/assay-mcp/src/server.rs
autonomous: true
must_haves:
  truths:
    - "spec_get accepts an optional resolve: bool parameter (default false)"
    - "When resolve is true, the response includes a resolved object with timeout cascade and working_dir validation"
    - "Timeout cascade always has the same shape: effective, spec (null), config, default fields"
    - "working_dir validation includes path, exists, and accessible fields"
    - "When resolve is false or omitted, no resolved block appears in the response"
    - "Config tier is null when no [gates] section exists"
  artifacts:
    - path: "crates/assay-mcp/src/server.rs"
      provides: "SpecGetParams with resolve field, resolved config block in spec_get handler"
  key_links:
    - from: "SpecGetParams.resolve"
      to: "resolved JSON block in spec_get response"
      via: "conditional JSON construction using config.gates and resolve_working_dir"
---

<objective>
Add resolved configuration display to the `spec_get` MCP tool.

When called with `resolve: true`, `spec_get` returns an additional `resolved` block showing the effective timeout with its 3-tier cascade (spec, config, default) and working_dir validation (path, exists, accessible). This gives agents visibility into how configuration precedence resolves without running a gate.
</objective>

<context>
@crates/assay-mcp/src/server.rs (lines 44-50 for SpecGetParams, lines 569-614 for spec_get handler, line 1410 for resolve_working_dir)
@.planning/phases/pending/38-observability-completion/38-RESEARCH.md (Timeout Three-Tier Precedence, Working Directory Resolution, Code Examples sections)
@.planning/phases/pending/38-observability-completion/38-CONTEXT.md (resolved config shape example)
</context>

<task type="auto">
  <name>Task 1: Add resolve parameter and resolved config block to spec_get</name>
  <files>crates/assay-mcp/src/server.rs</files>
  <action>
  1. Add `resolve: bool` field to `SpecGetParams`:
     ```rust
     #[schemars(description = "Include resolved configuration (effective timeouts, working_dir validation)")]
     #[serde(default)]
     pub resolve: bool,
     ```

  2. Update the `spec_get` tool description attribute to mention the resolve parameter:
     - Add: "Pass resolve=true to include effective timeout cascade (spec/config/default precedence) and working_dir validation."

  3. In the `spec_get` handler, after loading `config` and `entry`, add resolved config construction when `params.0.resolve` is true:
     - Extract `config_timeout`: `config.gates.as_ref().map(|g| g.default_timeout)`
     - Compute `effective_timeout`: `config_timeout.unwrap_or(300)`
     - Resolve working dir: `resolve_working_dir(&cwd, &config)`
     - Build the resolved JSON block:
       ```json
       {
         "timeout": {
           "effective": effective_timeout,
           "spec": null,
           "config": config_timeout,
           "default": 300
         },
         "working_dir": {
           "path": working_dir_display,
           "exists": working_dir.exists(),
           "accessible": working_dir.is_dir()
         }
       }
       ```
     - Merge the `"resolved"` key into both the Legacy and Directory response JSON objects conditionally (only when `resolve` is true).

  4. Use `serde_json::json!()` inline construction (consistent with existing `spec_get` pattern). Do NOT create a dedicated response struct.

  Implementation detail: Since both Legacy and Directory branches build a `serde_json::json!({...})` value, build the resolved block once before the match, then insert it into each response object. For example:
  ```rust
  let resolved_block = if params.0.resolve {
      let config_timeout = config.gates.as_ref().map(|g| g.default_timeout);
      let effective_timeout = config_timeout.unwrap_or(300);
      let working_dir = resolve_working_dir(&cwd, &config);
      Some(serde_json::json!({
          "timeout": {
              "effective": effective_timeout,
              "spec": serde_json::Value::Null,
              "config": config_timeout,
              "default": 300
          },
          "working_dir": {
              "path": working_dir.to_string_lossy(),
              "exists": working_dir.exists(),
              "accessible": working_dir.is_dir()
          }
      }))
  } else {
      None
  };
  ```
  Then in each match arm, after building the response `serde_json::Value`, conditionally insert:
  ```rust
  if let Some(resolved) = &resolved_block {
      response.as_object_mut().unwrap().insert("resolved".to_string(), resolved.clone());
  }
  ```
  </action>
  <verify>
  rtk cargo test -p assay-mcp
  rtk cargo clippy -p assay-mcp -- -D warnings
  </verify>
  <done>
  - SpecGetParams has a `resolve: bool` field with `#[serde(default)]`
  - spec_get handler conditionally adds `resolved` block with timeout cascade and working_dir validation
  - Timeout cascade has fixed shape: effective (u64), spec (null), config (Option<u64>), default (300)
  - working_dir block has: path (string), exists (bool), accessible (bool)
  - When resolve is false/omitted, response is identical to current behavior
  - All existing tests pass, clippy clean
  </done>
</task>

<task type="auto">
  <name>Task 2: Add tests for resolved config in spec_get</name>
  <files>crates/assay-mcp/src/server.rs</files>
  <action>
  Add unit/integration tests for the resolved config functionality. The test approach depends on what existing test patterns exist in the MCP crate. At minimum, verify:

  1. `SpecGetParams` deserializes correctly with `resolve: true` and with `resolve` omitted (defaults to false)
  2. The resolved block structure matches expectations when constructed

  If MCP handler integration tests exist (using test fixtures with `.assay/specs/` directories), add:
  - Test: spec_get with resolve=false returns no "resolved" key
  - Test: spec_get with resolve=true returns "resolved" with timeout and working_dir sub-objects
  - Test: timeout cascade shape — effective, spec (null), config, default are all present
  - Test: when config has no [gates] section, config tier is null

  If no integration test harness exists, add deserialization tests for `SpecGetParams` to confirm serde defaults work correctly.
  </action>
  <verify>
  rtk cargo test -p assay-mcp
  </verify>
  <done>
  - Tests verify SpecGetParams deserialization with and without resolve field
  - Tests verify resolved block structure when present
  - All tests pass
  </done>
</task>
