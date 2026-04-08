#!/usr/bin/env bash
# Setup script for the close-the-loop example.
#
# Copies spec files into .assay/specs/close-the-loop/ and resets any
# prior state (sessions, reviews) so the scenario starts clean.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SPECS_DIR="$PROJECT_ROOT/.assay/specs/close-the-loop"

echo "Setting up close-the-loop example..."

# Create spec directory.
mkdir -p "$SPECS_DIR"

# Copy spec + gates into the assay specs directory.
cp "$SCRIPT_DIR/spec.toml" "$SPECS_DIR/spec.toml"
cp "$SCRIPT_DIR/gates.toml" "$SPECS_DIR/gates.toml"

# Clean prior state for this spec only.
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

echo "Done. Spec installed at $SPECS_DIR"
echo "Prior sessions and reviews cleared."
