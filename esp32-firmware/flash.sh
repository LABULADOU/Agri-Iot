#!/data/data/com.termux/files/usr/bin/bash
# 农业物联网 ESP32 固件 - Termux 烧录脚本
# 使用方法：在 Termux 中运行 bash flash.sh

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${CYAN}"
echo "========================================="
echo "  农业物联网 ESP32 固件烧录工具 v1.0"
echo "========================================="
echo -e "${NC}"

# 检查依赖
check_deps() {
    echo -e "${YELLOW}[1/5] 检查依赖...${NC}"
    
    if ! command -v python &>/dev/null; then
        echo -e "${RED}未找到 Python，正在安装...${NC}"
        pkg install -y python
    fi
    
    if ! pip show esptool &>/dev/null; then
        echo -e "${YELLOW}未找到 esptool，正在安装...${NC}"
        pip install esptool
    fi
    
    echo -e "${GREEN}依赖检查完成${NC}"
}

# 编译固件
build_firmware() {
    echo -e "${YELLOW}[2/5] 编译固件...${NC}"
    
    if command -v pio &>/dev/null; then
        echo -e "${GREEN}使用 PlatformIO 编译...${NC}"
        pio run -e esp32dev
    else
        echo -e "${YELLOW}PlatformIO 未安装，使用预编译固件或手动编译${NC}"
        echo -e "${YELLOW}安装 PlatformIO: pip install platformio${NC}"
        return 1
    fi
}

# 查找串口设备
find_serial() {
    echo -e "${YELLOW}[3/5] 查找 ESP32 串口设备...${NC}"
    
    SERIAL_PORT=""
    
    # 检查可能的串口路径
    for port in /dev/ttyUSB* /dev/ttyACM* /dev/cu.SLAB* /dev/cu.usb*; do
        if [ -e "$port" ]; then
            SERIAL_PORT="$port"
            break
        fi
    done
    
    if [ -z "$SERIAL_PORT" ]; then
        echo -e "${RED}未找到串口设备！${NC}"
        echo -e "${YELLOW}请检查：${NC}"
        echo -e "  1. ESP32 是否通过 OTG 线连接到手机"
        echo -e "  2. 是否已授权 Termux 访问 USB 设备"
        echo -e "  3. 运行: ls /dev/tty* 查看可用设备"
        echo ""
        read -p "手动输入串口路径 (如 /dev/ttyUSB0): " SERIAL_PORT
        
        if [ ! -e "$SERIAL_PORT" ]; then
            echo -e "${RED}设备不存在: $SERIAL_PORT${NC}"
            exit 1
        fi
    else
        echo -e "${GREEN}找到串口: $SERIAL_PORT${NC}"
    fi
}

# 烧录固件
flash_firmware() {
    echo -e "${YELLOW}[4/5] 烧录固件到 ESP32...${NC}"
    
    # PlatformIO 烧录
    if command -v pio &>/dev/null; then
        echo -e "${GREEN}使用 PlatformIO 烧录...${NC}"
        pio run -e esp32dev --target upload --upload-port "$SERIAL_PORT"
    else
        # 手动 esptool 烧录
        FIRMWARE_PATH=".pio/build/esp32dev/firmware.bin"
        
        if [ ! -f "$FIRMWARE_PATH" ]; then
            echo -e "${RED}未找到固件文件: $FIRMWARE_PATH${NC}"
            exit 1
        fi
        
        echo -e "${CYAN}esptool.py --port $SERIAL_PORT --baud 921600 write_flash 0x0 $FIRMWARE_PATH${NC}"
        esptool.py --port "$SERIAL_PORT" --baud 921600 write_flash 0x0 "$FIRMWARE_PATH"
    fi
    
    echo -e "${GREEN}烧录完成！${NC}"
}

# 验证烧录
verify_flash() {
    echo -e "${YELLOW}[5/5] 验证烧录...${NC}"
    
    echo -e "${CYAN}打开串口监视器 (Ctrl+C 退出)...${NC}"
    echo ""
    
    if command -v pio &>/dev/null; then
        pio device monitor --port "$SERIAL_PORT" --baud 115200
    else
        python -m serial.tools.miniterm "$SERIAL_PORT" 115200
    fi
}

# 主流程
main() {
    check_deps
    
    echo ""
    echo -e "${YELLOW}选择操作：${NC}"
    echo "  1. 编译固件"
    echo "  2. 烧录固件"
    echo "  3. 编译并烧录"
    echo "  4. 串口监视器"
    read -p "选择 [1-4]: " choice
    
    case $choice in
        1)
            build_firmware
            ;;
        2)
            find_serial
            flash_firmware
            ;;
        3)
            build_firmware
            find_serial
            flash_firmware
            ;;
        4)
            find_serial
            verify_flash
            ;;
        *)
            echo -e "${RED}无效选择${NC}"
            exit 1
            ;;
    esac
    
    echo ""
    echo -e "${GREEN}操作完成！${NC}"
}

main "$@"
