---
created: 2026-03-01T04:30
title: Tighten deny.toml source controls from warn to deny
area: tooling
provenance: github:wollax/assay#16
files:
  - deny.toml:22-23
---

## Problem

`unknown-registry = "warn"` and `unknown-git = "warn"` neutralize supply chain source controls. A dependency from an untrusted registry or arbitrary git URL passes CI with only a warning.

## Solution

Change both to `"deny"` to enforce supply chain integrity. The existing `allow-registry` and `allow-git` lists will then act as proper allowlists.

## Resolution

Resolved during Phase 19-01. `deny.toml` uses `unknown-registry = "deny"`.
