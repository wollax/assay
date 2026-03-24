#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "Configuring git to use .githooks/ directory..."
git -C "$REPO_ROOT" config core.hooksPath .githooks

echo "Ensuring hooks are executable..."
chmod +x "$REPO_ROOT"/.githooks/pre-commit
chmod +x "$REPO_ROOT"/.githooks/pre-push

echo "✅ Git hooks installed. pre-commit and pre-push are now active."
