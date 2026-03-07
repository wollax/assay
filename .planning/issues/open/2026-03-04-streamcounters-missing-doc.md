---
created: 2026-03-04T10:00
title: StreamCounters missing doc comment — asymmetric with StreamConfig
area: assay-cli
severity: suggestion
files:
  - crates/assay-cli/src/main.rs
---

## Problem

`StreamCounters` struct lacks a doc comment, while its companion `StreamConfig` struct has documentation. This asymmetry reduces code clarity and doesn't follow documentation best practices.

## Solution

Add a doc comment to `StreamCounters` explaining its role in tracking gate execution statistics during streaming output.
