#!/usr/bin/env bash
# verify-docs.sh — Structural test for smelt-agent plugin documentation.
#
# Checks that MCP tool names referenced in plugin docs exist in the
# assay-mcp router (server.rs). Exits non-zero on any mismatch.
#
# Usage: bash plugins/smelt-agent/tests/verify-docs.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PLUGIN_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$PLUGIN_DIR/../.." && pwd)"

SERVER_RS="$REPO_ROOT/crates/assay-mcp/src/server.rs"

if [ ! -f "$SERVER_RS" ]; then
    echo "ERROR: server.rs not found at $SERVER_RS"
    exit 1
fi

# Extract MCP tool names from the router (pub async fn declarations in the
# #[tool_router] impl block). These are the canonical tool names.
ROUTER_TOOLS=$(grep 'pub async fn' "$SERVER_RS" \
    | grep -oE 'fn [a-z_]+' \
    | sed 's/fn //' \
    | grep -v '^serve$' \
    | sort -u)

# Extract tool names referenced in the MCP Tools table in AGENTS.md.
# Table rows look like: | `tool_name` | description |
DOC_TOOLS=$(grep -oE '`[a-z_]+`' "$PLUGIN_DIR/AGENTS.md" \
    | tr -d '`' \
    | sort -u)

# Known tools that exist on feature branches but not yet on main.
# These are documented in advance of the M015 merge and will be
# validated once M015 lands.
PENDING_TOOLS="poll_signals send_signal"

# Filter out non-tool identifiers (field names, config keys, etc.)
NON_TOOLS="run_id state_backend"

ERRORS=0

for tool in $DOC_TOOLS; do
    # Skip known non-tool identifiers
    skip=0
    for nt in $NON_TOOLS; do
        if [ "$tool" = "$nt" ]; then
            skip=1
            break
        fi
    done
    [ "$skip" -eq 1 ] && continue

    # Check if it's a pending tool (on a feature branch, not yet merged)
    pending=0
    for pt in $PENDING_TOOLS; do
        if [ "$tool" = "$pt" ]; then
            pending=1
            break
        fi
    done

    if [ "$pending" -eq 1 ]; then
        echo "  PENDING: $tool (M015 feature branch — not yet on main)"
        continue
    fi

    # Check if tool exists in router
    if ! echo "$ROUTER_TOOLS" | grep -qx "$tool"; then
        echo "  MISSING: $tool (referenced in docs but not in router)"
        ERRORS=$((ERRORS + 1))
    fi
done

# Also check skill files for tool references
for skill_file in "$PLUGIN_DIR"/skills/*.md; do
    SKILL_TOOLS=$(grep -oE '`(poll_signals|send_signal|merge_propose|orchestrate_run|run_manifest|orchestrate_status|gate_run|spec_list|spec_get)`' "$skill_file" 2>/dev/null | tr -d '`' | sort -u || true)
    for tool in $SKILL_TOOLS; do
        pending=0
        for pt in $PENDING_TOOLS; do
            if [ "$tool" = "$pt" ]; then
                pending=1
                break
            fi
        done
        [ "$pending" -eq 1 ] && continue

        if ! echo "$ROUTER_TOOLS" | grep -qx "$tool"; then
            echo "  MISSING: $tool (referenced in $(basename "$skill_file") but not in router)"
            ERRORS=$((ERRORS + 1))
        fi
    done
done

if [ "$ERRORS" -gt 0 ]; then
    echo ""
    echo "FAIL: $ERRORS tool name(s) referenced in docs but missing from router"
    exit 1
fi

echo "OK: all documented tool names verified"
exit 0
