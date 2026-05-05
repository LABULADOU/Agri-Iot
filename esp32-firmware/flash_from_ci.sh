#!/data/data/com.termux/files/usr/bin/bash
# 农业物联网 ESP32 固件 - Termux 烧录脚本
# 用途：从 GitHub Actions 下载预编译固件并烧录到 ESP32
# 在 Termux 中运行：bash flash_from_ci.sh

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

echo -e "${CYAN}${BOLD}"
echo "============================================"
echo "  农业物联网 ESP32 固件 - 下载+烧录"
echo "  开发板: ESP32-DevKit-32E"
echo "============================================"
echo -e "${NC}"

echo -e "${YELLOW}[1/4] 安装依赖...${NC}"
pkg update -y 2>/dev/null
pkg install python curl esptool -y 2>/dev/null
pip install --break-system-packages pyserial 2>/dev/null || pip install pyserial 2>/dev/null

echo -e "${YELLOW}[2/4] 下载预编译固件...${NC}"

# 从最新的 CI 构建下载固件
# 如果是你自己的仓库，修改下面的 URL
DOWNLOAD_URL=""

if [ -n "$1" ]; then
    DOWNLOAD_URL="$1"
else
    echo -e "${CYAN}请输入固件下载链接（GitHub Actions Artifact 的直接链接）:${NC}"
    echo -e "${YELLOW}提示：在 GitHub Actions -> 构建任务 -> Artifacts 中右键下载链接${NC}"
    read -p "下载链接: " DOWNLOAD_URL
fi

if [ -z "$DOWNLOAD_URL" ]; then
    echo -e "${RED}错误：未提供下载链接${NC}"
    exit 1
fi

FIRMWARE_PATH="/tmp/esp32-firmware.bin"
curl -L -o "$FIRMWARE_PATH" "$DOWNLOAD_URL"

if [ ! -f "$FIRMWARE_PATH" ]; then
    echo -e "${RED}下载失败${NC}"
    exit 1
fi

FIRMWARE_SIZE=$(stat -c%s "$FIRMWARE_PATH" 2>/dev/null || stat -f%z "$FIRMWARE_PATH" 2>/dev/null)
echo -e "${GREEN}下载完成! 固件大小: $((FIRMWARE_SIZE / 1024)) KB${NC}"

echo -e "${YELLOW}[3/4] 查找 ESP32 串口...${NC}"
PORT=""

for dev in /dev/ttyUSB* /dev/ttyACM* /dev/cu.SLAB* /dev/cu.usb*; do
    if [ -e "$dev" ]; then
        PORT="$dev"
        break
    fi
done

if [ -z "$PORT" ]; then
    echo -e "${RED}未找到 ESP32 串口设备!${NC}"
    echo -e "${YELLOW}请确认：${NC}"
    echo -e "  1. ESP32 已通过 USB OTG 连接到手机"
    echo -e "  2. 数据线支持数据传输（不只是充电）"
    echo ""
    ls /dev/tty* 2>/dev/null | grep -E "USB|ACM|SLAB|usb|cu" || echo "  无可用设备"
    echo ""
    read -p "手动输入串口路径 (如 /dev/ttyUSB0): " PORT
    if [ ! -e "$PORT" ]; then
        echo -e "${RED}设备不存在: $PORT${NC}"
        exit 1
    fi
else
    echo -e "${GREEN}找到串口: $PORT${NC}"
fi

echo -e "${YELLOW}[4/4] 烧录固件...${NC}"
echo -e "${CYAN}波特率: 921600${NC}"

esptool.py --port "$PORT" --baud 921600 write_flash 0x0 "$FIRMWARE_PATH"

echo -e "${GREEN}烧录完成！${NC}"
echo ""
echo -e "${CYAN}重启 ESP32 后可以在串口监视器查看输出：${NC}"
echo -e "${YELLOW}python -m serial.tools.miniterm $PORT 115200${NC}"
