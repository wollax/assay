#!/usr/bin/env bash
# Reset the close-the-loop example to a runnable state.
#
# Restores spec.toml status to in-progress (undoing any auto-promotion)
# and clears sessions/reviews so both paths can be re-run.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SPECS_DIR="$PROJECT_ROOT/.assay/specs/close-the-loop"

if [ ! -d "$SPECS_DIR" ]; then
    echo "Spec not installed. Run setup.sh first."
    exit 1
fi

# Re-copy spec.toml to restore status = "in-progress".
cp "$SCRIPT_DIR/spec.toml" "$SPECS_DIR/spec.toml"

# Clear sessions and reviews for this spec only.
if [ -d "$PROJECT_ROOT/.assay/sessions" ]; then
    for f in "$PROJECT_ROOT/.assay/sessions"/*.json; do
        [ -f "$f" ] || continue
        if grep -q '"spec_name":"close-the-loop"' "$f" 2>/dev/null || \
           grep -q '"spec_name": "close-the-loop"' "$f" 2>/dev/null; then
            rm -f "$f"
        fi
    done
fi
rm -rf "$PROJECT_ROOT/.assay/reviews/close-the-loop/"

echo "Reset complete. Status restored to in-progress, state cleared."
