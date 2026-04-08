#!/usr/bin/env bash
# Run the abort path: agent exceeds tool budget → checkpoint fails → early abort.
#
# Prerequisites:
#   1. Run setup.sh (or reset.sh) first
#   2. claude CLI must be installed and authenticated
#
# Expected outcome:
#   - Checkpoint fires after 2 tool calls
#   - EventCount criterion fails (count=2 > max=1)
#   - Agent subprocess is killed (SIGTERM → SIGKILL)
#   - Spec remains in "in-progress" status
#   - assay spec review close-the-loop shows the failed checkpoint
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "=== Abort Path ==="
echo "Running manifest with misbehaving prompt (expects checkpoint failure)..."
echo ""

cd "$PROJECT_ROOT"
assay run "$SCRIPT_DIR/manifest-abort.toml" --timeout 120 || true

echo ""
echo "=== Review ==="
assay spec review close-the-loop || true
