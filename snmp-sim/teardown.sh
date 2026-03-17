#!/bin/bash
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "==> Stopping snmpsim agents..."
for pid_file in "$SCRIPT_DIR"/.*.pid; do
    [ -f "$pid_file" ] && kill "$(cat "$pid_file")" 2>/dev/null && rm "$pid_file"
done

echo "==> Removing pf rules..."
sudo pfctl -a com.apple/snmpsim -F all 2>/dev/null || true

echo "==> Removing IP aliases..."
sudo ifconfig lo0 -alias 10.99.0.10
sudo ifconfig lo0 -alias 10.99.0.11
sudo ifconfig lo0 -alias 10.99.0.12
sudo ifconfig lo0 -alias 10.99.0.13

echo "==> Done."
