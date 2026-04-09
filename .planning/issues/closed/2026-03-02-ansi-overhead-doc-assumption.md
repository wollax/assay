---
title: Document ANSI_COLOR_OVERHEAD assumptions
area: assay-cli
priority: low
source: Phase 9 PR review
---

## Problem

The `ANSI_COLOR_OVERHEAD` constant has a doc comment but doesn't clarify its assumptions. It assumes 2-digit SGR (Select Graphic Rendition) codes. Extended sequences like 256-color (`\x1b[38;5;{n}m`) or truecolor (`\x1b[38;2;r;g;bm`) would require different overhead values. Future contributors may apply this constant incorrectly if they add support for these extended color modes.

## Solution

Update the `ANSI_COLOR_OVERHEAD` doc comment to explicitly note that it assumes standard 2-digit SGR codes and explain what would need to change if extended sequences (256-color, truecolor) are added in the future.
