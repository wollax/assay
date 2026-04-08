#!/usr/bin/env bash
# Run the clean path: agent stays within tool budget → auto-promotes to verified.
#
# Prerequisites:
#   1. Run setup.sh (or reset.sh) first
#   2. claude CLI must be installed and authenticated
#   3. spec.toml status must be "in-progress" (reset.sh restores this)
#
# Expected outcome:
#   - Agent completes in ≤1 tool call → checkpoint never fires
#   - Session-end NoToolErrors criterion passes
#   - auto_promote triggers: in-progress → verified
#   - assay spec review shows "Auto-promotion: in-progress → verified"
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "=== Clean Path (auto-promote) ==="
echo "Running manifest with clean prompt (expects successful promotion)..."
echo ""

cd "$PROJECT_ROOT"
assay run "$SCRIPT_DIR/manifest.toml" --timeout 120

echo ""
echo "=== Review ==="
assay spec review close-the-loop
