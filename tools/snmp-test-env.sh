#!/bin/bash
set -euo pipefail

# SNMP Test Environment — manages 6 snmpd instances on loopback aliases
# Usage: sudo tools/snmp-test-env.sh up|down
#        tools/snmp-test-env.sh status

SNMPD=/opt/homebrew/opt/net-snmp/sbin/snmpd
SNMPGET=/opt/homebrew/opt/net-snmp/bin/snmpget
HOSTS=(10.99.0.10 10.99.0.11 10.99.0.12 10.99.0.13 10.99.0.14 10.99.0.15)
PID_DIR=/tmp/snmp-test-env
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CONF_DIR="$SCRIPT_DIR/snmp-test-configs"

# Community strings per host (parallel array)
COMMUNITIES=(netdefault netdefault secret42 secret42 public netdefault)
SYSNAMES=("switch-core-01" "switch-access-01" "router-gw-01" "firewall-01" "printer-lobby" "ap-wireless-01")

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

check_netsnmp() {
    if [ ! -x "$SNMPD" ]; then
        echo "ERROR: net-snmp not found at $SNMPD"
        echo "Install with: brew install net-snmp"
        exit 1
    fi
}

prepare_configs() {
    # Replace CONF_DIR placeholder in config files with actual path
    for host in "${HOSTS[@]}"; do
        local conf="$CONF_DIR/snmpd-${host}.conf"
        local runtime="$PID_DIR/snmpd-${host}.conf"
        if [ ! -f "$conf" ]; then
            echo "ERROR: Config file not found: $conf"
            exit 1
        fi
        sed "s|CONF_DIR|$CONF_DIR|g" "$conf" > "$runtime"
    done
}

cmd_up() {
    check_netsnmp

    # Create loopback aliases (idempotent)
    echo "Setting up loopback aliases..."
    for host in "${HOSTS[@]}"; do
        if ifconfig lo0 | grep -q "inet $host "; then
            echo "  $host already exists"
        else
            ifconfig lo0 alias "$host" netmask 255.255.255.0
            echo "  $host added"
        fi
    done

    # Create PID dir and prepare runtime configs
    mkdir -p "$PID_DIR"
    prepare_configs

    # Make handler executable
    chmod +x "$CONF_DIR/snmp-pass-handler.sh"

    # Start snmpd instances
    echo ""
    echo "Starting snmpd instances..."
    for i in "${!HOSTS[@]}"; do
        local host="${HOSTS[$i]}"
        local pidfile="$PID_DIR/snmpd-${host}.pid"
        local runtime_conf="$PID_DIR/snmpd-${host}.conf"

        if [ -f "$pidfile" ] && kill -0 "$(cat "$pidfile")" 2>/dev/null; then
            echo "  $host already running (PID $(cat "$pidfile"))"
            continue
        fi

        # Clean stale PID file
        rm -f "$pidfile"

        "$SNMPD" -C -c "$runtime_conf" -I -ifTable,-ifXTable -p "$pidfile" -Lf /dev/null
        echo "  $host started (PID $(cat "$pidfile"))"
    done

    # Verify each instance
    echo ""
    echo "Verifying..."
    sleep 1
    local all_ok=true
    for i in "${!HOSTS[@]}"; do
        local host="${HOSTS[$i]}"
        local community="${COMMUNITIES[$i]}"
        local expected="${SYSNAMES[$i]}"
        local result
        result=$("$SNMPGET" -v2c -c "$community" -t 2 -r 1 "$host" sysName.0 2>/dev/null | sed 's/.*= STRING: //' || echo "FAILED")
        if echo "$result" | grep -q "$expected"; then
            printf "  ${GREEN}✓${NC} %-14s  %-20s  community=%-12s\n" "$host" "$expected" "$community"
        else
            printf "  ${RED}✗${NC} %-14s  expected=%-20s  got=%s\n" "$host" "$expected" "$result"
            all_ok=false
        fi
    done

    echo ""
    if $all_ok; then
        printf "${GREEN}All 6 SNMP test hosts are running.${NC}\n"
    else
        printf "${YELLOW}Some hosts failed verification — check config files.${NC}\n"
    fi
}

cmd_down() {
    echo "Stopping snmpd instances..."
    for host in "${HOSTS[@]}"; do
        local pidfile="$PID_DIR/snmpd-${host}.pid"
        if [ -f "$pidfile" ]; then
            local pid
            pid=$(cat "$pidfile")
            if kill -0 "$pid" 2>/dev/null; then
                kill "$pid"
                echo "  $host stopped (PID $pid)"
            else
                echo "  $host was not running"
            fi
            rm -f "$pidfile"
        else
            echo "  $host no PID file"
        fi
    done

    # Remove runtime configs
    rm -f "$PID_DIR"/snmpd-*.conf

    # Remove loopback aliases
    echo ""
    echo "Removing loopback aliases..."
    for host in "${HOSTS[@]}"; do
        if ifconfig lo0 | grep -q "inet $host "; then
            ifconfig lo0 -alias "$host"
            echo "  $host removed"
        else
            echo "  $host not present"
        fi
    done

    # Clean up PID dir if empty
    rmdir "$PID_DIR" 2>/dev/null || true

    echo ""
    printf "${GREEN}SNMP test environment torn down.${NC}\n"
}

cmd_status() {
    echo "SNMP Test Environment Status"
    echo "=============================="
    printf "%-16s  %-22s  %-12s  %-10s  %s\n" "IP" "sysName" "community" "alias" "process"
    printf "%-16s  %-22s  %-12s  %-10s  %s\n" "---" "-------" "---------" "-----" "-------"

    for i in "${!HOSTS[@]}"; do
        local host="${HOSTS[$i]}"
        local community="${COMMUNITIES[$i]}"
        local sysname="${SYSNAMES[$i]}"
        local pidfile="$PID_DIR/snmpd-${host}.pid"

        local alias_status="${RED}down${NC}"
        if ifconfig lo0 2>/dev/null | grep -q "inet $host "; then
            alias_status="${GREEN}up${NC}"
        fi

        local proc_status="${RED}down${NC}"
        if [ -f "$pidfile" ] && kill -0 "$(cat "$pidfile")" 2>/dev/null; then
            proc_status="${GREEN}up${NC}"
        fi

        printf "%-16s  %-22s  %-12s  ${alias_status}%-4s  ${proc_status}%s\n" \
            "$host" "$sysname" "$community" "" ""
    done
}

case "${1:-}" in
    up)
        cmd_up
        ;;
    down)
        cmd_down
        ;;
    status)
        cmd_status
        ;;
    *)
        echo "Usage: $0 {up|down|status}"
        echo ""
        echo "  up     — Create loopback aliases and start snmpd instances (requires sudo)"
        echo "  down   — Stop snmpd instances and remove aliases (requires sudo)"
        echo "  status — Show current state (no sudo needed)"
        exit 1
        ;;
esac
