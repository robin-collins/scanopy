#!/bin/bash
set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SNMP_PORT=1161
PYTHON="$SCRIPT_DIR/.venv/bin/python3"
AGENT="$SCRIPT_DIR/run-agent.py"
PF_ANCHOR="com.apple/snmpsim"

echo "==> Creating IP aliases on lo0..."
sudo ifconfig lo0 alias 10.99.0.10 netmask 255.255.255.0
sudo ifconfig lo0 alias 10.99.0.11 netmask 255.255.255.0
sudo ifconfig lo0 alias 10.99.0.12 netmask 255.255.255.0
sudo ifconfig lo0 alias 10.99.0.13 netmask 255.255.255.0

if [ ! -d "$SCRIPT_DIR/.venv" ]; then
    echo "==> Creating Python venv and installing snmpsim..."
    python3 -m venv "$SCRIPT_DIR/.venv"
    "$SCRIPT_DIR/.venv/bin/pip" install -q snmpsim-lextudio
fi

echo "==> Starting snmpsim agents on port $SNMP_PORT..."
"$PYTHON" "$AGENT" --data-dir "$SCRIPT_DIR/snmprec/router" --agent-udpv4-endpoint "10.99.0.10:$SNMP_PORT" --logging-method null --log-level error &
echo $! > "$SCRIPT_DIR/.router.pid"

"$PYTHON" "$AGENT" --data-dir "$SCRIPT_DIR/snmprec/switch" --agent-udpv4-endpoint "10.99.0.11:$SNMP_PORT" --logging-method null --log-level error &
echo $! > "$SCRIPT_DIR/.switch.pid"

"$PYTHON" "$AGENT" --data-dir "$SCRIPT_DIR/snmprec/host" --agent-udpv4-endpoint "10.99.0.12:$SNMP_PORT" --logging-method null --log-level error &
echo $! > "$SCRIPT_DIR/.host.pid"

"$PYTHON" "$AGENT" --data-dir "$SCRIPT_DIR/snmprec/dist-router" --agent-udpv4-endpoint "10.99.0.13:$SNMP_PORT" --logging-method null --log-level error &
echo $! > "$SCRIPT_DIR/.dist-router.pid"

echo "==> Setting up port forwarding 161 -> $SNMP_PORT..."
cat <<EOF | sudo pfctl -a "$PF_ANCHOR" -f -
rdr pass on lo0 proto udp from any to 10.99.0.10 port 161 -> 10.99.0.10 port $SNMP_PORT
rdr pass on lo0 proto udp from any to 10.99.0.11 port 161 -> 10.99.0.11 port $SNMP_PORT
rdr pass on lo0 proto udp from any to 10.99.0.12 port 161 -> 10.99.0.12 port $SNMP_PORT
rdr pass on lo0 proto udp from any to 10.99.0.13 port 161 -> 10.99.0.13 port $SNMP_PORT
EOF
sudo pfctl -e 2>/dev/null || true

echo "==> Verifying..."
sleep 2
snmpget -v2c -c public "10.99.0.10:$SNMP_PORT" 1.3.6.1.2.1.1.5.0 && echo "  Router (direct) OK"
snmpget -v2c -c public 10.99.0.10 1.3.6.1.2.1.1.5.0 && echo "  Router (port 161) OK" || echo "  WARNING: port 161 forwarding not working"

echo ""
echo "==> SNMP simulation running."
echo "    Teardown:  ./teardown.sh"
