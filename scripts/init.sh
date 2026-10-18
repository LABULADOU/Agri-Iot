#!/bin/bash
# scripts/init.sh - 轻量级进程管理器
# 托管 agri-mqtt-broker + agri-server，任一挂掉自动重启

set -euo pipefail

# === 配置 ===
SCRIPT_DIR="$(cd "$(dirname "$(readlink -f "$0")")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

BUILD_TYPE="${BUILD_TYPE:-debug}"
BROKER_BIN="$ROOT_DIR/target/$BUILD_TYPE/broker"
SERVER_BIN="$ROOT_DIR/target/$BUILD_TYPE/agri-server"
SERVER_ENV="MQTT_BROKER_ADDR=127.0.0.1:1883 RUST_LOG=info,agri_mqtt=info,agri_server=info"

LOG_DIR="$ROOT_DIR/logs"
mkdir -p "$LOG_DIR"

CHECK_INTERVAL=5
COOLDOWN=10
MAX_RETRIES=5
LOG_MAX_BYTES=$((200 * 1024 * 1024))   # rotate when log exceeds 200MB
LOG_KEEP_BYTES=$((20 * 1024 * 1024))    # keep last 20MB

# === 全局变量 ===
BROKER_PID=""
SERVER_PID=""
BROKER_FAIL_COUNT=0
SERVER_FAIL_COUNT=0

# === 辅助函数 ===
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [INIT] $1" | tee -a "$LOG_DIR/init.log"
}

wait_for_port() {
    local port=$1
    local timeout=${2:-10}
    log "Waiting for port $port to be ready..."
    for i in $(seq 1 "$timeout"); do
        if command -v nc &>/dev/null && nc -z localhost "$port" 2>/dev/null; then
            return 0
        fi
        if [ -e "/proc/net/tcp" ] && grep -qi "$(printf '%04X' "$port")" /proc/net/tcp 2>/dev/null; then
            return 0
        fi
        sleep 1
    done
    log "WARNING: Port $port did not become ready in ${timeout}s"
    return 1
}

check_alive() {
    local pid=${1:-}
    if [ -z "$pid" ]; then
        return 1
    fi
    kill -0 "$pid" 2>/dev/null
}

# 原地截断轮转：子进程以 O_APPEND 打开同一 inode，
# truncate 后新写入继续追加到文件末尾，无需重启进程。
rotate_log_if_big() {
    local f="$1"
    [ -f "$f" ] || return 0
    local size
    size=$(stat -c%s "$f" 2>/dev/null || echo 0)
    if [ "$size" -gt "$LOG_MAX_BYTES" ]; then
        tail -c "$LOG_KEEP_BYTES" "$f" > "$f.tmp" 2>/dev/null || true
        cat "$f.tmp" > "$f"
        rm -f "$f.tmp"
        log "Rotated $f ($((size / 1024 / 1024))MB -> $((LOG_KEEP_BYTES / 1024 / 1024))MB kept)"
    fi
}

# === 进程管理 ===
start_broker() {
    log "Starting agri-mqtt-broker..."
    $BROKER_BIN >> "$LOG_DIR/broker.log" 2>&1 &
    BROKER_PID=$!
    echo "$BROKER_PID" > /tmp/agri-mqtt-broker.pid
    log "Broker started with PID $BROKER_PID"
    wait_for_port 1883 10
}

start_server() {
    log "Starting agri-server..."
    eval "$SERVER_ENV $SERVER_BIN >> '$LOG_DIR/server.log' 2>&1 &"
    SERVER_PID=$!
    echo "$SERVER_PID" > /tmp/agri-server.pid
    log "Server started with PID $SERVER_PID"
}

# === 信号处理 ===
cleanup() {
    log "Received shutdown signal. Stopping children..."
    kill -TERM ${BROKER_PID:-} ${SERVER_PID:-} 2>/dev/null || true
    sleep 2
    kill -KILL ${BROKER_PID:-} ${SERVER_PID:-} 2>/dev/null || true
    wait ${BROKER_PID:-} 2>/dev/null || true
    wait ${SERVER_PID:-} 2>/dev/null || true
    rm -f /tmp/agri-*.pid
    log "All processes stopped. Exiting."
    exit 0
}

trap cleanup SIGTERM SIGINT SIGQUIT SIGHUP

# === 主循环 ===
main() {
    log "Agri-System Init starting (build=$BUILD_TYPE)"

    if [ ! -x "$BROKER_BIN" ]; then
        log "FATAL: Broker binary not found at $BROKER_BIN (run 'cargo build -p agri-mqtt' first)"
        exit 1
    fi
    if [ ! -x "$SERVER_BIN" ]; then
        log "FATAL: Server binary not found at $SERVER_BIN (run 'cargo build -p agri-server' first)"
        exit 1
    fi

    rotate_log_if_big "$LOG_DIR/broker.log"
    rotate_log_if_big "$LOG_DIR/server.log"

    start_broker
    start_server

    while true; do
        rotate_log_if_big "$LOG_DIR/broker.log"
        rotate_log_if_big "$LOG_DIR/server.log"
        rotate_log_if_big "$LOG_DIR/init.log"

        # --- 检查 Broker ---
        if ! check_alive "$BROKER_PID"; then
            wait ${BROKER_PID:-} 2>/dev/null || true
            BROKER_FAIL_COUNT=$((BROKER_FAIL_COUNT + 1))
            if [ "$BROKER_FAIL_COUNT" -gt "$MAX_RETRIES" ]; then
                log "CRITICAL: Broker failed $MAX_RETRIES times continuously. Giving up."
                exit 1
            fi
            log "Broker died (count: $BROKER_FAIL_COUNT). Restarting in ${COOLDOWN}s..."
            sleep "$COOLDOWN"
            start_broker
        else
            if [ "$BROKER_FAIL_COUNT" -gt 0 ]; then
                BROKER_FAIL_COUNT=0
            fi
        fi

        # --- 检查 Server ---
        if ! check_alive "$SERVER_PID"; then
            wait ${SERVER_PID:-} 2>/dev/null || true
            SERVER_FAIL_COUNT=$((SERVER_FAIL_COUNT + 1))
            if [ "$SERVER_FAIL_COUNT" -gt "$MAX_RETRIES" ]; then
                log "CRITICAL: Server failed $MAX_RETRIES times continuously. Giving up."
                exit 1
            fi
            log "Server died (count: $SERVER_FAIL_COUNT). Restarting in ${COOLDOWN}s..."
            sleep "$COOLDOWN"
            start_server
        else
            if [ "$SERVER_FAIL_COUNT" -gt 0 ]; then
                SERVER_FAIL_COUNT=0
            fi
        fi

        sleep "$CHECK_INTERVAL"
    done
}

main
