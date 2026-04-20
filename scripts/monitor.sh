#!/usr/bin/env bash
# monitor.sh — Run espflash monitor on /dev/ttyUSB0, tee output to a log file.
#
# Usage:  ./scripts/monitor.sh [log file]
# Default log: /tmp/flashpoint-monitor.log

set -euo pipefail

PORT="${ESPFLASH_PORT:-/dev/ttyUSB0}"
LOG="${1:-/tmp/flashpoint-monitor.log}"

echo "[monitor] Logging to: $LOG"
echo "[monitor] Port: $PORT"
echo "[monitor] Press Ctrl+C to stop."
echo "---" | tee "$LOG"

espflash monitor --port "$PORT" 2>&1 | tee -a "$LOG"
