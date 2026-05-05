#!/data/data/com.termux/files/usr/bin/bash
# 快速烧录脚本 - 适用于已编译好的固件

set -e

echo "=== ESP32 快速烧录 ==="
echo ""

# 1. 检查 esptool
if ! command -v esptool.py &>/dev/null; then
    echo "安装 esptool..."
    pip install esptool
fi

# 2. 查找串口
echo "查找串口..."
PORT=$(ls /dev/ttyUSB* 2>/dev/null | head -1)
if [ -z "$PORT" ]; then
    PORT=$(ls /dev/ttyACM* 2>/dev/null | head -1)
fi

if [ -z "$PORT" ]; then
    echo "未找到串口设备"
    echo "可用设备:"
    ls /dev/tty* 2>/dev/null | grep -E "USB|ACM|SLAB"
    read -p "手动输入串口路径: " PORT
fi

echo "使用串口: $PORT"

# 3. 烧录
echo "烧录固件..."
esptool.py --port "$PORT" --baud 921600 write_flash 0x0 firmware.bin

echo ""
echo "烧录完成！"
echo ""
echo "查看串口输出："
python -m serial.tools.miniterm "$PORT" 115200
