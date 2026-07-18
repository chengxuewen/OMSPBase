#!/usr/bin/env bash
# pixi-init.sh — Bootstrap OMSPBase development environment
# Installs pixi binary, project dependencies, and toolchain
# Usage: scripts/pixi-init.sh [--root-dir <path>]
set -euo pipefail

SCRIPT_DIR_PINIT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
source "${SCRIPT_DIR_PINIT}/_common.sh"

# --- Config ---
PIXI_VERSION="${PIXI_VERSION:-0.67.2}"
# Default download mirror (Gitee for China, GitHub.com for global)
PIXI_REPOURL="${PIXI_REPOURL:-https://gitee.com/chengxuewen-github/pixi}"
PIXI_OFFICIAL_URL="https://github.com/prefix-dev/pixi/releases/download"

# --- Args ---
arg_root_dir="${PROJECT_ROOT}"
while [ $# -gt 0 ]; do
    case "$1" in
        --root-dir) arg_root_dir="$2"; shift 2 ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

echo "=== OMSPBase pixi environment setup ==="
echo "Project root: ${PROJECT_ROOT}"
echo "Install root: ${arg_root_dir}"

# --- Step 1: Install pixi binary ---
need_install=false
if [ ! -f "${PIXI_BIN}" ]; then
    need_install=true
else
    installed_ver="$("${PIXI_BIN}" --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo '0.0.0')"
    if [ "$installed_ver" != "$PIXI_VERSION" ]; then
        echo "pixi version mismatch: $installed_ver (need $PIXI_VERSION), reinstalling..."
        need_install=true
    fi
fi

if $need_install; then
    echo "Installing pixi ${PIXI_VERSION}..."

    # Try official installer first, fall back to manual download
    if [ -f "${SCRIPT_DIR_PINIT}/pixi-install.sh" ]; then
        PIXI_VERSION="$PIXI_VERSION" bash "${SCRIPT_DIR_PINIT}/pixi-install.sh"
    else
        # Direct download
        os="$(uname -s | tr '[:upper:]' '[:lower:]')"
        case "$os" in darwin) os="apple-darwin" ;; linux) os="unknown-linux-musl" ;; esac
        fname="pixi-${ARCH}-${os}.tar.gz"

        mkdir -p "$(dirname "${PIXI_BIN}")"
        if curl -fsSL "${PIXI_OFFICIAL_URL}/v${PIXI_VERSION}/${fname}" -o /tmp/pixi.tar.gz 2>/dev/null; then
            tar xzf /tmp/pixi.tar.gz -C "$(dirname "${PIXI_BIN}")"
        elif curl -fsSL "${PIXI_REPOURL}/releases/download/v${PIXI_VERSION}/${fname}" -o /tmp/pixi.tar.gz 2>/dev/null; then
            tar xzf /tmp/pixi.tar.gz -C "$(dirname "${PIXI_BIN}")"
        else
            echo "ERROR: Failed to download pixi. Set PIXI_REPOURL for mirrors."
            exit 1
        fi
        chmod +x "${PIXI_BIN}"
        rm -f /tmp/pixi.tar.gz
    fi
    echo "pixi ${PIXI_VERSION} installed at ${PIXI_BIN}"
fi

# --- Step 2: Set up pixi cache ---
mkdir -p "${PIXI_CACHE_DIR}"
export PIXI_CACHE_DIR

# --- Step 3: Install project dependencies ---
echo "Installing project dependencies..."
cd "${PROJECT_ROOT}"

if ! "${PIXI_BIN}" install --manifest-path "${PROJECT_ROOT}/pixi.toml" 2>/dev/null; then
    echo "pixi install failed. Regenerating lock file..."
    "${PIXI_BIN}" update --manifest-path "${PROJECT_ROOT}/pixi.toml"
    "${PIXI_BIN}" install --manifest-path "${PROJECT_ROOT}/pixi.toml"
fi

echo ""
echo "=== OMSPBase pixi environment ready ==="
echo "Activate with: source pixi.sh"
echo "Or run tasks:   pixi run build | pixi run test | pixi run lint"
echo ""
echo "Quick start:"
echo "  source pixi.sh          # activate environment"
echo "  pixi run build          # build workspace"
echo "  pixi run test           # run tests"
echo "  pixi run lint           # clippy + fmt check"
echo "  pixi run format-fix     # auto-format"
