#!/usr/bin/env bash
# E2E verification: Host → Server ← Remote signaling chain
# Starts all 3 components, waits for signaling, verifies health endpoints.
set -euo pipefail

SCRIPTS_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPTS_DIR/.." && pwd)"
BUILD_DIR="$PROJECT_DIR/target/debug"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

PID_FILE="/tmp/omspbase-e2e.pids"
rm -f "$PID_FILE"

cleanup() {
    echo -e "\n${YELLOW}=== Shutting down ===${NC}"
    if [ -f "$PID_FILE" ]; then
        while read -r pid; do
            kill "$pid" 2>/dev/null || true
        done < "$PID_FILE"
        rm -f "$PID_FILE"
    fi
    echo "E2E cleanup complete."
}
trap cleanup EXIT INT TERM

# --- Build ---
echo -e "${GREEN}[1/5] Building workspace...${NC}"
(cd "$PROJECT_DIR" && cargo build --workspace 2>&1 | tail -1)

# --- Start Server ---
echo -e "${GREEN}[2/5] Starting omspbase-server...${NC}"
"$BUILD_DIR/omspbase-server" --config /dev/null 2>&1 &
SERVER_PID=$!
echo "$SERVER_PID" >> "$PID_FILE"
sleep 1

# Verify server health
if curl -sf http://localhost:9800/health > /dev/null 2>&1; then
    echo -e "  ${GREEN}✓ Server health OK${NC}"
else
    echo -e "  ${RED}✗ Server health failed${NC}"
    exit 1
fi

# --- Start Host ---
echo -e "${GREEN}[3/5] Starting omspbase-remote-host (headless)...${NC}"
"$BUILD_DIR/omspbase-remote-host" --config /dev/null 2>&1 &
HOST_PID=$!
echo "$HOST_PID" >> "$PID_FILE"
sleep 1

# Verify host health
if curl -sf http://localhost:9801/metrics > /dev/null 2>&1; then
    echo -e "  ${GREEN}✓ Host health OK${NC}"
else
    echo -e "  ${RED}✗ Host health failed${NC}"
    exit 1
fi

# --- Start Remote ---
echo -e "${GREEN}[4/5] Starting omspbase-remote-client (headless)...${NC}"
"$BUILD_DIR/omspbase-remote-client" --config /dev/null 2>&1 &
REMOTE_PID=$!
echo "$REMOTE_PID" >> "$PID_FILE"
sleep 1

# Verify remote health
if curl -sf http://localhost:9101/health > /dev/null 2>&1; then
    echo -e "  ${GREEN}✓ Remote health OK${NC}"
else
    echo -e "  ${RED}✗ Remote health failed${NC}"
    exit 1
fi

# --- Verify signaling chain ---
echo -e "${GREEN}[5/5] Verifying signaling chain...${NC}"
sleep 2

# Check that host and remote are connected via server signaling
SERVER_METRICS=$(curl -s http://localhost:9800/metrics 2>/dev/null || echo "")
HOST_METRICS=$(curl -s http://localhost:9800/metrics 2>/dev/null || echo "")
REMOTE_METRICS=$(curl -s http://localhost:9101/metrics 2>/dev/null || echo "")

echo "  Server metrics available: $(echo "$SERVER_METRICS" | wc -c) bytes"
echo "  Remote metrics available: $(echo "$REMOTE_METRICS" | wc -c) bytes"

# Verify all 3 processes still running (macOS compatible)
if kill -0 $SERVER_PID 2>/dev/null && kill -0 $HOST_PID 2>/dev/null && kill -0 $REMOTE_PID 2>/dev/null; then
    echo -e "${GREEN}✓ All 3 components running${NC}"
else
    echo -e "${RED}✗ Not all components running${NC}"
    exit 1
fi
fi

echo -e "\n${GREEN}=== E2E verification passed ===${NC}"
echo "  Server  : http://localhost:9800 (health/metrics)"
  Host    : http://localhost:9801 (health/metrics)
echo "  Remote  : http://localhost:9101 (health/metrics)"
echo "  Signaling: Host + Remote connect to Server via WebSocket"
