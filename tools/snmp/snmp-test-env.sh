#!/bin/bash
set -euo pipefail

# SNMP Test Environment — manages 6 snmpd containers on a Docker network
# Subnet: 10.99.0.0/24 (10.99.0.10–10.99.0.15)
# Usage: tools/snmp/snmp-test-env.sh up|down|status

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SNMPGET="${SNMPGET:-/opt/homebrew/opt/net-snmp/bin/snmpget}"

HOSTS=(10.99.0.10 10.99.0.11 10.99.0.12 10.99.0.13 10.99.0.14 10.99.0.15)
COMMUNITIES=(netdefault netdefault secret42 secret42 public netdefault)
SYSNAMES=("switch-core-01" "switch-access-01" "router-gw-01" "firewall-01" "printer-lobby" "ap-wireless-01")

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

cmd_up() {
    echo "Building and starting SNMP test containers..."
    docker compose -f "$SCRIPT_DIR/docker-compose.yml" up -d --build --quiet-pull

    echo ""
    echo "Verifying..."
    sleep 3
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
        echo ""
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo "  Subnet: 10.99.0.0/24 (10.99.0.10–10.99.0.15)"
        echo ""
        printf "  %-16s %-22s %s\n" "IP" "Host" "Community"
        printf "  %-16s %-22s %s\n" "──────────────" "────────────────────" "────────────"
        for i in "${!HOSTS[@]}"; do
            printf "  %-16s %-22s %s\n" "${HOSTS[$i]}" "${SYSNAMES[$i]}" "${COMMUNITIES[$i]}"
        done
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    else
        printf "${YELLOW}Some hosts failed verification — check container logs with: docker compose -f $SCRIPT_DIR/docker-compose.yml logs${NC}\n"
    fi
}

cmd_down() {
    echo "Stopping SNMP test containers..."
    docker compose -f "$SCRIPT_DIR/docker-compose.yml" down
    printf "${GREEN}SNMP test environment torn down.${NC}\n"
}

cmd_status() {
    echo "SNMP Test Environment Status"
    echo "=============================="
    docker compose -f "$SCRIPT_DIR/docker-compose.yml" ps
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
        echo "  up     — Build and start snmpd containers"
        echo "  down   — Stop and remove containers"
        echo "  status — Show container status"
        exit 1
        ;;
esac
