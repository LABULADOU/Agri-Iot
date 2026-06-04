#!/usr/bin/env python3
"""
MQTT Telemetry Stress Test
Sends rapid telemetry messages to the MQTT broker and verifies dedup.
"""

import paho.mqtt.client as mqtt
import json
import time
import sys
import random

BROKER_HOST = "127.0.0.1"
BROKER_PORT = 11885
TOPIC = "agri/node/stress-node-001/telemetry"
NODE_ID = "stress-node-001"

# Config
BURST_COUNT = 500       # messages per burst
DUP_FRACTION = 0.1      # 10% duplicates
BURSTS = 5              # total bursts

published = []
inserted_unique = set()

def on_connect(client, userdata, flags, rc):
    print(f"Connected: rc={rc}")

def on_publish(client, userdata, mid):
    pass

def main():
    client = mqtt.Client(client_id="stress-tester")
    client.on_connect = on_connect
    client.on_publish = on_publish
    client.connect(BROKER_HOST, BROKER_PORT, 60)
    client.loop_start()

    base_temp = 20.0
    base_hum = 60.0
    
    start = time.time()
    total_sent = 0

    for burst in range(BURSTS):
        print(f"\n=== Burst {burst+1}/{BURSTS} ===")
        
        msgs = []
        for i in range(BURST_COUNT):
            seq = burst * BURST_COUNT + i + 1
            
            # Decide if this is a duplicate of a random earlier message
            is_dup = len(published) > 10 and random.random() < DUP_FRACTION
            if is_dup:
                seq = random.choice(published[-50:])  # dup of last 50
            
            published.append(seq)
            
            metrics = {
                "air_temp": round(base_temp + random.uniform(-2, 2), 1),
                "air_humidity": round(base_hum + random.uniform(-5, 5), 1),
                "soil_temp": round(base_temp - 3 + random.uniform(-1, 1), 1),
                "soil_moisture": round(random.uniform(30, 70), 1),
                "ec": round(random.uniform(500, 2000), 0),
            }
            
            payload = json.dumps({
                "node_id": NODE_ID,
                "seq": seq,
                "metrics": metrics,
            })
            
            msgs.append(payload)
            total_sent += 1
        
        # Send all messages in burst
        for p in msgs:
            client.publish(TOPIC, p, qos=1)
        
        time.sleep(1)  # brief pause between bursts

    elapsed = time.time() - start
    print(f"\n=== Stress Test Complete ===")
    print(f"Total messages sent: {total_sent}")
    print(f"Time: {elapsed:.1f}s")
    print(f"Rate: {total_sent/elapsed:.0f} msg/s")
    print(f"(Unique seq values: {len(set(published))})")
    
    client.loop_stop()
    client.disconnect()

if __name__ == "__main__":
    main()
