# cupel path dependency needs release-path TODO

**Area:** Cargo.toml:12
**Severity:** Low
**Source:** PR #125 review (code-reviewer)

## Description

`cupel = { path = "../cupel/crates/cupel" }` is a local path dependency. Before any public release, this must be changed to a crates.io or git dep.
