# deny.toml skip entries should have cleanup TODOs

**Source:** PR review (Phase 19)
**Area:** deny.toml
**Priority:** low

Skip entries for crossterm 0.28, rustix 0.38, etc. should include comments like `# TODO: remove when crossterm upgraded to 0.29+` to make cleanup triggers explicit.
