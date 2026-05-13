#!/bin/bash
set -e

echo "========================================="
echo "  农业物联网项目 - 文件完整性检查"
echo "========================================="
echo ""

PASS=0
FAIL=0

check_file() {
    if [ -f "$1" ]; then
        echo "✓ $1"
        PASS=$((PASS + 1))
    else
        echo "✗ $1 (MISSING)"
        FAIL=$((FAIL + 1))
    fi
}

echo "--- Workspace ---"
check_file "Cargo.toml"
check_file ".env.example"

echo ""
echo "--- agri-core ---"
check_file "agri-core/Cargo.toml"
check_file "agri-core/src/lib.rs"
check_file "agri-core/src/models.rs"
check_file "agri-core/src/db.rs"
check_file "agri-core/src/error.rs"

echo ""
echo "--- agri-server ---"
check_file "agri-server/Cargo.toml"
check_file "agri-server/src/main.rs"
check_file "agri-server/src/routes.rs"
check_file "agri-server/src/state.rs"
check_file "agri-server/src/request_logger.rs"
check_file "agri-server/src/rule_engine.rs"

echo ""
echo "--- agri-mqtt ---"
check_file "agri-mqtt/Cargo.toml"
check_file "agri-mqtt/src/lib.rs"
check_file "agri-mqtt/src/broker.rs"
check_file "agri-mqtt/src/client.rs"
check_file "agri-mqtt/src/handler.rs"

echo ""
echo "--- agri-ui ---"
check_file "agri-ui/package.json"
check_file "agri-ui/index.html"
check_file "agri-ui/vite.config.ts"
check_file "agri-ui/src/main.tsx"
check_file "agri-ui/src/App.tsx"
check_file "agri-ui/src/types/index.ts"
check_file "agri-ui/src/services/api.ts"

echo ""
echo "--- migrations ---"
check_file "migrations/001_init.sql"

echo ""
echo "--- deploy ---"
check_file "deploy/build.sh"
check_file "deploy/run.sh"

echo ""
echo "--- esp32-firmware ---"
check_file "esp32-firmware/main.ino"

echo ""
echo "========================================="
echo "  结果: $PASS 通过, $FAIL 失败"
echo "========================================="
