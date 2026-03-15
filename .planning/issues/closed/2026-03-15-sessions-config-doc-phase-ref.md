# `SessionsConfig` Doc Uses Lowercase `agent_running` Instead of Enum Variant Reference

## Description

The doc comment for `SessionsConfig` (or one of its fields) refers to the phase as `agent_running` in plain lowercase text. The correct reference is `SessionPhase::AgentRunning`. Using the enum variant name (ideally as a rustdoc intra-doc link: `` [`SessionPhase::AgentRunning`] ``) keeps the doc accurate if the variant is ever renamed and makes it navigable via `rustdoc`.

## File Reference

`crates/assay-types/src/lib.rs` — `SessionsConfig` doc comment

## Category

documentation / accuracy
