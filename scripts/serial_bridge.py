#!/usr/bin/env python3
"""
农业物联网 - 串口转 HTTP 桥接工具
从 ESP32 串口读取 JSON 遥测数据，转发到后端 API

使用前准备：
  pip install pyserial requests

使用方法（Windows - COM3）：
  python serial_bridge.py COM3

使用方法（Linux - /dev/ttyUSB0）：
  python serial_bridge.py /dev/ttyUSB0
"""

import sys
import json
import time
import requests
import serial
import serial.tools.list_ports

# 后端地址（按实际修改）
SERVER_URL = "http://127.0.0.1:3001"
API_TELEMETRY = f"{SERVER_URL}/api/v1/telemetry"

# 串口配置
BAUD_RATE = 115200
TIMEOUT = 2


def find_serial_ports():
    ports = serial.tools.list_ports.comports()
    return [p.device for p in ports]


def read_serial(port_name):
    ser = serial.Serial(port_name, BAUD_RATE, timeout=TIMEOUT)
    ser.flushInput()
    print(f"[+] 已打开串口: {port_name} @ {BAUD_RATE} baud")
    print(f"[>] 转发目标: {API_TELEMETRY}")
    print("=" * 60)
    return ser


def parse_json_line(line):
    try:
        return json.loads(line)
    except json.JSONDecodeError:
        return None


def send_telemetry(data):
    try:
        resp = requests.post(API_TELEMETRY, json=data, timeout=3)
        if resp.status_code == 200:
            result = resp.json()
            return result.get("inserted", 0)
        else:
            print(f"[-] HTTP {resp.status_code}: {resp.text}")
            return -1
    except requests.exceptions.ConnectionError:
        print(f"[-] 无法连接到服务器 {SERVER_URL}")
        return -1
    except Exception as e:
        print(f"[-] 请求异常: {e}")
        return -1


def main():
    if len(sys.argv) < 2:
        print("=" * 60)
        print("  农业物联网 - 串口桥接工具")
        print("=" * 60)
        print()
        available = find_serial_ports()
        if available:
            print("可用的串口:")
            for p in available:
                print(f"  - {p}")
        print()
        print("用法:")
        print(f"  python {sys.argv[0]} <串口号>")
        print(f"  示例: python {sys.argv[0]} COM3")
        print(f"  示例: python {sys.argv[0]} /dev/ttyUSB0")
        sys.exit(1)

    port = sys.argv[1]
    stats = {"sent": 0, "failed": 0, "last_time": None}

    ser = read_serial(port)
    buf = ""

    print("等待 JSON 数据... (Ctrl+C 退出)\n")

    try:
        while True:
            char = ser.read(1).decode("utf-8", errors="ignore")
            if char == "":
                continue

            if char == "\n":
                data = None
                # 提取 ---DATA 前缀后的 JSON
                if buf.startswith("---DATA "):
                    json_str = buf[8:]
                    data = parse_json_line(json_str)
                else:
                    data = parse_json_line(buf)
                buf = ""

                if data and "metrics" in data and "node_id" in data:
                    ts = time.strftime("%H:%M:%S")
                    node = data["node_id"]
                    m = data["metrics"]
                    metrics_str = ", ".join(
                        f"{k}={v}" for k, v in m.items()
                        if k not in ("relay_state", "rssi")
                    )

                    count = send_telemetry({
                        "node_id": node,
                        "metrics": m,
                    })

                    if count > 0:
                        stats["sent"] += 1
                        stats["last_time"] = ts
                        print(f"[{ts}] {node} | {metrics_str} | ✅ 写入{count}条")
                    else:
                        stats["failed"] += 1
                        print(f"[{ts}] {node} | {metrics_str} | ❌ 转发失败")
                elif data:
                    # 非遥测 JSON，跳过
                    pass
            else:
                buf += char

    except KeyboardInterrupt:
        print("\n\n=== 统计 ===")
        print(f"  成功发送: {stats['sent']}")
        print(f"  发送失败: {stats['failed']}")
        print(f"  最后发送: {stats['last_time'] or '无'}")
    finally:
        ser.close()
        print("[*] 串口已关闭")


if __name__ == "__main__":
    main()
