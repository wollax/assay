---
created: 2026-03-16T15:30
title: extract_path_from_message missing "modified in" pattern
area: core
provenance: local
files:
  - crates/assay-core/src/merge.rs:178-196
---

## Problem

`extract_path_from_message` handles "deleted in" but not the symmetric "modified in" pattern. The message `"file.rs modified in HEAD and deleted in feature"` falls through to the fallback and returns the entire message as the path.

## Solution

Add a handler for `" modified in "` pattern, extracting the path prefix before it.
