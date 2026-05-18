#!/usr/bin/env python3
import time, json, random, urllib.request

API = "http://127.0.0.1:3001/api/v1/telemetry"
NODE_ID = "esp32-node-001"

print(f"[*] 通过 HTTP 推送遥测数据到 {API}")
print(f"{'-'*60}")
print(f"{'时间':<20} | {'土壤水分':<8} | {'温度':<6} | {'湿度':<6} | {'光照':<6}")
print(f"{'-'*60}")

while True:
    data = {
        "node_id": NODE_ID,
        "metrics": {
            "soil_moisture": round(random.uniform(30, 80), 1),
            "temperature": round(random.uniform(15, 35), 1),
            "humidity": round(random.uniform(40, 90), 1),
            "light": round(random.uniform(200, 1000), 0),
        }
    }
    req = urllib.request.Request(
        API,
        data=json.dumps(data).encode(),
        headers={"Content-Type": "application/json"},
        method="POST"
    )
    try:
        resp = urllib.request.urlopen(req, timeout=5)
        result = json.loads(resp.read())
        ts = time.strftime("%H:%M:%S")
        m = data["metrics"]
        print(f"{ts} | {m['soil_moisture']}%      | {m['temperature']}°C   | {m['humidity']}%    | {m['light']}")
    except Exception as e:
        print(f"[-] 错误: {e}")

    time.sleep(5)
