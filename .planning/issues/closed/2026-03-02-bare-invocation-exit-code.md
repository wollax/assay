---
title: Bare assay invocation should exit with non-zero code
area: assay-cli
priority: low
source: Phase 9 PR review
---

## Problem

Running `assay` bare (outside a project) exits with code 0. Tools like `git` exit with non-zero status codes when invoked in invalid contexts. An exit code of 0 signals success to automation tools and CI pipelines, which may cause them to treat the failure as a success. This is problematic for scripting and automation.

## Solution

Consider whether `assay` invoked bare should exit with code 1 (or another non-zero code) to signal an error. This aligns with standard UNIX conventions and makes the tool more reliable for use in scripts and pipelines.

## Resolution

Resolved during Phase 18-01. Returns `Ok(1)` for bare invocation outside project.
