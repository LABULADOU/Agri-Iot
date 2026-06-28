#!/bin/bash
# OTA 一键部署脚本
# 用法: ./scripts/ota_deploy.sh <node-001|node-002> [--lan] [--no-trigger]
# 示例:
#   ./scripts/ota_deploy.sh node-001              # 默认 Funnel（公网/局域网均可用）
#   ./scripts/ota_deploy.sh node-001 --lan         # 局域网 HTTP 加速（仅同子网可用）
#   ./scripts/ota_deploy.sh node-001 --no-trigger  # 仅构建部署，不触发 OTA

set -euo pipefail

TARGET=""
USE_LAN=false
NO_TRIGGER=false
for arg in "$@"; do
    case "$arg" in
        --lan) USE_LAN=true ;;
        --no-trigger) NO_TRIGGER=true ;;
        -*)
            echo "未知选项: $arg"
            echo "用法: $0 <node-001|node-002> [--lan] [--no-trigger]"
            exit 1
            ;;
        *)
            if [[ -z "$TARGET" ]]; then
                TARGET="$arg"
            else
                echo "未知参数: $arg"
                exit 1
            fi
            ;;
    esac
done

if [[ -z "$TARGET" ]]; then
    echo "用法: $0 <node-001|node-002> [--lan] [--no-trigger]"
    exit 1
fi

ENV="esp32-${TARGET}"
FW_BIN=".pio/build/${ENV}/firmware.bin"
FW_VER=$(date +%Y%m%d-%H%M%S)
SSH_TARGET="zero"
PROJ_DIR="~/Agri-Iot/esp32-firmware"
LOCAL_FW_DIR="/root/agri-iot/agri-server/static/firmware"
PRIVATE_KEY="/root/agri-iot/esp32-firmware/keys/ota_private.pem"

if [[ "$USE_LAN" == true ]]; then
    STATIC_URL="http://172.20.10.2:3001/firmware"
else
    STATIC_URL="https://debian.taile2b316.ts.net/firmware"
fi

NODE_ID=""
DEVICE_UUID=""
case "$TARGET" in
    node-001) NODE_ID="esp32-node-001"; DEVICE_UUID="9b590407-cb98-4292-ae55-3ab78891fac6" ;;
    node-002) NODE_ID="esp32-node-002"; DEVICE_UUID="13728da5-b4c5-41cd-b791-ee6266d65c64" ;;
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

# 5. 触发 OTA（通过 HTTP API，broker 自动转发 MQTT）
if [[ "$NO_TRIGGER" == true ]]; then
    echo "--- 跳过触发 (--no-trigger) ---"
else
    echo "--- 发送 OTA 命令 ---"
    API_URL="http://127.0.0.1:3001/api/v1/devices/${DEVICE_UUID}/command"
    OTA_PAYLOAD="{\"command\":\"ota\",\"params\":{\"url\":\"${STATIC_URL}/${FW_FILE}\",\"sig\":\"${SIG}\"}}"
    echo "URL: ${API_URL}"
    echo "Payload: ${OTA_PAYLOAD}"
    RESULT=$(curl -s -X POST "${API_URL}" -H "Content-Type: application/json" -d "${OTA_PAYLOAD}")
    echo "响应: ${RESULT}"
    if echo "${RESULT}" | grep -q '"id"'; then
        echo "✅ OTA 命令已发送! 节点 ${NODE_ID} 将下载并更新."
    else
        echo "⚠️  OTA 命令可能失败，请检查响应."
    fi
fi

echo "=== 完成 ==="
