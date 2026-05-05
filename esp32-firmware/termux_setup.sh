#!/data/data/com.termux/files/usr/bin/bash
# 农业物联网 ESP32 固件 - Termux 一键安装+编译+烧录
# 在 Android Termux 中运行：bash termux_setup.sh

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

echo -e "${CYAN}${BOLD}"
echo "============================================"
echo "  农业物联网 ESP32 固件 - Termux 一键烧录"
echo "  开发板: ESP32-DevKit-32E"
echo "  固件: WiFi + MQTT + 传感器 + 继电器"
echo "============================================"
echo -e "${NC}"

# 配置
WIFI_SSID="ChinaNet-Jj7u"
WIFI_PASS="bp9tu6fm"
MQTT_SERVER="192.168.1.6"
MQTT_PORT=1883
NODE_ID="esp32-node-001"

echo -e "${YELLOW}[1/6] 安装依赖...${NC}"
pkg update -y 2>/dev/null
pkg install python git make curl tar -y 2>/dev/null

echo -e "${YELLOW}[2/6] 安装 esptool 和 PlatformIO...${NC}"
pip install --break-system-packages esptool platformio pyserial 2>/dev/null || \
pip install esptool platformio pyserial 2>/dev/null

echo -e "${GREEN}依赖安装完成${NC}"

# 查找 ESP32 固件目录
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
if [ ! -f "$SCRIPT_DIR/main.ino" ]; then
    echo -e "${RED}错误: 未找到 main.ino，请在 esp32-firmware 目录中运行此脚本${NC}"
    exit 1
fi

cd "$SCRIPT_DIR"

echo -e "${YELLOW}[3/6] 编译固件...${NC}"
pio run -e esp32dev

echo -e "${GREEN}编译完成!${NC}"

FIRMWARE_PATH=".pio/build/esp32dev/firmware.bin"
if [ ! -f "$FIRMWARE_PATH" ]; then
    echo -e "${RED}错误: 固件文件不存在: $FIRMWARE_PATH${NC}"
    exit 1
fi

FIRMWARE_SIZE=$(stat -c%s "$FIRMWARE_PATH" 2>/dev/null || stat -f%z "$FIRMWARE_PATH" 2>/dev/null)
echo -e "${CYAN}固件大小: $((FIRMWARE_SIZE / 1024)) KB${NC}"

echo -e "${YELLOW}[4/6] 查找 ESP32 串口...${NC}"
PORT=""

# 查找串口
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
    echo -e "  3. 按住 BOOT 键插入 USB 进入烧录模式"
    echo ""
    echo -e "${YELLOW}可用串口设备：${NC}"
    ls /dev/tty* 2>/dev/null | grep -E "USB|ACM|SLAB|usb|cu" || echo "  无"
    echo ""
    read -p "手动输入串口路径 (如 /dev/ttyUSB0): " PORT
    if [ ! -e "$PORT" ]; then
        echo -e "${RED}设备不存在: $PORT${NC}"
        exit 1
    fi
else
    echo -e "${GREEN}找到串口: $PORT${NC}"
fi

echo -e "${YELLOW}[5/6] 烧录固件到 ESP32...${NC}"
echo -e "${CYAN}波特率: 921600${NC}"
esptool.py --port "$PORT" --baud 921600 write_flash 0x0 "$FIRMWARE_PATH"

echo -e "${GREEN}烧录完成！${NC}"

echo -e "${YELLOW}[6/6] 查看串口输出 (Ctrl+C 退出)...${NC}"
echo ""
python -m serial.tools.miniterm "$PORT" 115200
