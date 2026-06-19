#!/bin/bash
# OTA 一键部署脚本
# 用法: ./scripts/ota_deploy.sh <node-001|node-002> [--no-trigger]
# 示例: ./scripts/ota_deploy.sh node-001

set -euo pipefail

TARGET="${1:-}"
NO_TRIGGER="${2:-}"
if [[ -z "$TARGET" ]]; then
    echo "用法: $0 <node-001|node-002> [--no-trigger]"
    exit 1
fi

ENV="esp32-${TARGET}"
FW_BIN=".pio/build/${ENV}/firmware.bin"
FW_VER=$(date +%Y%m%d-%H%M%S)
SSH_TARGET="zero"
PROJ_DIR="~/Agri-Iot/esp32-firmware"
LOCAL_FW_DIR="/root/agri-iot/agri-server/static/firmware"
PRIVATE_KEY="/root/agri-iot/esp32-firmware/keys/ota_private.pem"
STATIC_URL="https://debian.taile2b316.ts.net/firmware"

NODE_ID=""
case "$TARGET" in
    node-001) NODE_ID="esp32-node-001" ;;
    node-002) NODE_ID="esp32-node-002" ;;
    *) echo "未知目标: $TARGET (可选: node-001, node-002)"; exit 1 ;;
esac

echo "=== OTA 部署: ${NODE_ID} (${FW_VER}) ==="

# 1. 构建
echo "--- 构建 ${ENV} ---"
ssh "${SSH_TARGET}" \
    "export PATH=\$PATH:/home/admino/.local/bin && cd ${PROJ_DIR} && pio run -e ${ENV}" 2>&1 | tail -3

# 2. 复制固件到本地
echo "--- 获取 firmware.bin ---"
scp \
    "${SSH_TARGET}:${PROJ_DIR}/${FW_BIN}" \
    "/tmp/${ENV}.bin"

# 3. 签名
echo "--- ECDSA 签名 ---"
SIG=$(openssl dgst -sha256 -sign "${PRIVATE_KEY}" "/tmp/${ENV}.bin" | base64 -w0)
echo "签名完成 (${#SIG} bytes)"

# 4. 部署到服务器
echo "--- 部署到服务器 ---"
FW_FILE="firmware-${TARGET}-${FW_VER}.bin"
cp "/tmp/${ENV}.bin" "${LOCAL_FW_DIR}/${FW_FILE}"
ln -sf "${FW_FILE}" "${LOCAL_FW_DIR}/firmware-${TARGET}.bin"
echo "固件: ${STATIC_URL}/${FW_FILE}"

# 5. 触发 OTA（通过本地 MQTT broker）
if [[ "$NO_TRIGGER" == "--no-trigger" ]]; then
    echo "--- 跳过触发 (--no-trigger) ---"
else
    echo "--- 发送 OTA 命令 ---"
    OTA_CMD="{\"command\":\"ota\",\"params\":{\"url\":\"${STATIC_URL}/${FW_FILE}\",\"sig\":\"${SIG}\"}}"
    echo "命令: ${OTA_CMD}"
    mosquitto_pub -h 127.0.0.1 -p 1883 -t "agri/node/${NODE_ID}/command/ota" -m "${OTA_CMD}"
    echo "已发送! 节点 ${NODE_ID} 将下载并更新."
fi

echo "=== 完成 ==="
