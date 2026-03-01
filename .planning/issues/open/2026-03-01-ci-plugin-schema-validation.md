---
created: 2026-03-01T04:30
title: CI plugin validation should check schema not just JSON syntax
area: tooling
provenance: github:wollax/assay#14
files:
  - .github/workflows/ci.yml:27-35
---

## Problem

CI plugin validation steps only check if files are parseable JSON via `json.load()`. They don't validate semantic correctness (required fields, correct types). A `plugin.json` with missing required fields passes CI, giving a false sense of safety.

## Solution

Either rename CI steps to "Check parseable JSON (schema validation pending)" for honesty, or add JSON Schema validation using `check-jsonschema` once plugin format schemas are available. Also add `encoding='utf-8'` to `open()` calls.
