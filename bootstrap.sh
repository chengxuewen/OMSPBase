#!/usr/bin/env bash
# bootstrap.sh — First-time setup for OMSPBase development
# Usage: source bootstrap.sh
#
# This is the user-facing entry point. Run once per machine:
#   source bootstrap.sh
# After initial setup, use:
#   source pixi.sh
set -euo pipefail

BOOTSTRAP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"

echo "================================================"
echo "  OMSPBase Development Environment Bootstrap"
echo "================================================"
echo ""

# Step 1: Install pixi + project dependencies
echo "[1/2] Installing pixi and project dependencies..."
bash "${BOOTSTRAP_DIR}/scripts/pixi-init.sh"

echo ""

# Step 2: Activate pixi environment
echo "[2/2] Activating pixi environment..."
source "${BOOTSTRAP_DIR}/scripts/pixi-shell.sh"

echo ""
echo "================================================"
echo "  OMSPBase environment ready!"
echo "================================================"
echo ""
echo "Next time, just run:  source pixi.sh"
