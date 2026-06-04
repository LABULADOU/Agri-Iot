#!/bin/bash
# 用法：sudo ./run_bridge.sh [串口设备]
# 需要 sudo 权限访问串口设备
# 建议：在 /etc/sudoers 中添加 "username ALL=(ALL) NOPASSWD: /usr/bin/python3 /path/to/serial_bridge.py"
DEVICE="${1:-/dev/ttyUSB0}"
exec sudo python3 "$(dirname "$0")/serial_bridge.py" "$DEVICE"
