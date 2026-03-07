---
created: 2026-03-01T06:00
title: Identify dogfooding checkpoint — use Assay to build Assay
area: workflow
phase: null
provenance: kata-plan-phase:phase-2-planning-session
files: []
---

## Problem

Assay is a spec-driven quality gate tool, but it's being built without using itself. The sooner Assay dogfoods its own development, the sooner real usage exposes design issues, missing features, and workflow friction that synthetic testing won't catch.

## Solution

Introduce a dogfooding checkpoint once enough infrastructure exists. The natural progression:

**Phase 2 GO (MCP works):** MCP server can talk to Claude Code — prerequisite for everything.

**After Phases 6-7 (spec + gate evaluation):** Write an Assay spec for Assay itself (e.g., `just ready` as a command gate, formatting checks as criteria). Run `assay gate run` during development to validate builds. This is the earliest viable dogfooding point.

**After Phase 8 (MCP server tools):** Claude Code can call `spec_get` and `gate_run` programmatically. Agents building Assay can query its own gates.

**After Phase 10 (plugin):** Full dogfooding — the PostToolUse hook auto-runs gates after every code change, and the Stop hook prevents completion without passing gates. Assay enforces its own quality during its own development.

## Recommendation

- After Phase 7 completes, create a self-referential spec (`.assay/specs/self.toml`) with criteria like `just ready`, `just fmt-check`, `just lint`
- Wire it into the development workflow before building Phase 8+
- Track friction points as issues — they represent real user experience problems
