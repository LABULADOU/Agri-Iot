#!/usr/bin/env python3
"""
农业物联网 ESP32 节点模拟器
用于在真实硬件烧录前验证 MQTT Broker、后端 API 和 Dashboard 的完整性。
"""

import time
import json
import random
import paho.mqtt.client as mqtt

# 配置
MQTT_HOST = "127.0.0.1"
MQTT_PORT = 1883
NODE_ID = "esp32-node-001"
TOPIC = f"agri/{NODE_ID}/telemetry"

print(f"[*] 正在连接 MQTT Broker (localhost:{MQTT_PORT})...")

def on_connect(client, userdata, flags, rc):
    if rc == 0:
        print(f"[+] 已连接，开始发布遥测数据到主题: {TOPIC}")
    else:
        print(f"[-] 连接失败，代码: {rc}")

client = mqtt.Client()
client.on_connect = on_connect

try:
    client.connect(MQTT_HOST, MQTT_PORT, 60)
    client.loop_start()
    
    print(f"{'-'*50}")
    print(f"{'时间':<20} | {'土壤水分':<8} | {'温度':<6} | {'湿度':<6} | {'光照':<6}")
    print(f"{'-'*50}")
    
    while True:
        data = {
            "node_id": NODE_ID,
            "soil_moisture": round(random.uniform(30, 80), 1),
            "temperature": round(random.uniform(15, 35), 1),
            "humidity": round(random.uniform(40, 90), 1),
            "light_intensity": round(random.uniform(200, 1000), 0),
            "timestamp": int(time.time())
        }
        
        payload = json.dumps(data)
        client.publish(TOPIC, payload)
        
        ts = time.strftime("%H:%M:%S")
        print(f"{ts} | {data['soil_moisture']}%      | {data['temperature']}°C   | {data['humidity']}%    | {data['light_intensity']}")
        
        time.sleep(5)
        
except ConnectionRefusedError:
    print("[-] 无法连接到 MQTT Broker。请确保 agri-server 已启动。")
except KeyboardInterrupt:
    print("\n[*] 模拟已停止。")
    client.loop_stop()
    client.disconnect()
