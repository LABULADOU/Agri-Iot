/*
 * 农业物联网 ESP32 固件 v1.0
 * 功能：DHT22 温湿度采集 + MQTT 上报（云 Broker） + 远程控制
 */

#include <WiFi.h>
#include <PubSubClient.h>
#include <ArduinoJson.h>
#include <DHT.h>

// ==================== 配置（首次使用请修改） ====================

// WiFi 配置 — 手机热点或路由器
const char* WIFI_SSID = "YOUR_WIFI_SSID";
const char* WIFI_PASSWORD = "YOUR_WIFI_PASSWORD";

// MQTT 配置 — 云 Broker
const char* MQTT_SERVER = "broker.emqx.io";
const int MQTT_PORT = 1883;
const char* NODE_ID = "esp32-node-001";

// 引脚定义
#define DHTPIN 15               // DHT22 数据引脚
#define DHTTYPE DHT22           // DHT22 温湿度传感器
#define RELAY_PIN 16            // 继电器控制引脚

// 采集间隔 (毫秒)
const unsigned long READ_INTERVAL = 10000;

// ==================== 全局变量 ====================

WiFiClient espClient;
PubSubClient mqtt(espClient);
DHT dht(DHTPIN, DHTTYPE);

unsigned long lastRead = 0;
bool relayState = false;

// ==================== 初始化 ====================

void setup() {
    Serial.begin(115200);
    Serial.println("\n=== Agri-IoT ESP32 DHT22 ===");

    pinMode(RELAY_PIN, OUTPUT);
    digitalWrite(RELAY_PIN, LOW);

    dht.begin();
    setupWiFi();
    setupMQTT();
}

// ==================== WiFi ====================

void setupWiFi() {
    Serial.printf("Connecting WiFi: %s\n", WIFI_SSID);
    WiFi.begin(WIFI_SSID, WIFI_PASSWORD);
    WiFi.setAutoReconnect(true);

    int timeout = 20;
    while (WiFi.status() != WL_CONNECTED && timeout > 0) {
        delay(500);
        Serial.print(".");
        timeout--;
    }

    if (WiFi.status() == WL_CONNECTED) {
        Serial.printf("\nWiFi connected! IP: %s\n", WiFi.localIP().toString().c_str());
    } else {
        Serial.println("\nWiFi failed, deep sleeping...");
        ESP.deepSleep(0);
    }
}

// ==================== MQTT ====================

void setupMQTT() {
    mqtt.setServer(MQTT_SERVER, MQTT_PORT);
    mqtt.setCallback(onCommand);
    mqtt.setKeepAlive(30);
}

void reconnectMQTT() {
    while (!mqtt.connected()) {
        Serial.print("Connecting MQTT...");

        String willTopic = String("agri/node/") + NODE_ID + "/status";

        if (mqtt.connect(NODE_ID, willTopic.c_str(), 1, true, "offline")) {
            Serial.println(" connected");

            mqtt.publish(willTopic.c_str(), "online");

            String cmdTopic = String("agri/node/") + NODE_ID + "/command/#";
            mqtt.subscribe(cmdTopic.c_str());
            Serial.printf(" Subscribed: %s\n", cmdTopic.c_str());
        } else {
            Serial.printf(" failed (rc=%d), retry in 5s...\n", mqtt.state());
            delay(5000);
        }
    }
}

// ==================== 主循环 ====================

void loop() {
    if (!mqtt.connected()) {
        reconnectMQTT();
    }
    mqtt.loop();

    unsigned long now = millis();
    if (now - lastRead >= READ_INTERVAL) {
        publishTelemetry();
        lastRead = now;
    }
}

// ==================== 数据采集与上报 ====================

void publishTelemetry() {
    StaticJsonDocument<192> doc;
    JsonObject metrics = doc.createNestedObject("metrics");

    // DHT22 温湿度
    float temp = dht.readTemperature();
    float hum = dht.readHumidity();

    if (!isnan(temp)) {
        metrics["temperature"] = round(temp * 100) / 100.0;
        Serial.printf("Temp: %.1fC | ", temp);
    }
    if (!isnan(hum)) {
        metrics["humidity"] = round(hum * 100) / 100.0;
        Serial.printf("Hum: %.1f%% | ", hum);
    }

    // 网络信号
    metrics["rssi"] = WiFi.RSSI();

    char buf[256];
    serializeJson(doc, buf);

    String topic = String("agri/node/") + NODE_ID + "/telemetry";
    bool ok = mqtt.publish(topic.c_str(), buf);

    Serial.printf("Publish: %s\n", ok ? "OK" : "FAIL");
}

// ==================== 控制指令处理 ====================

void onCommand(char* topic, byte* payload, unsigned int length) {
    String msg;
    for (unsigned int i = 0; i < length; i++) {
        msg += (char)payload[i];
    }

    StaticJsonDocument<128> doc;
    DeserializationError err = deserializeJson(doc, msg);
    if (err) {
        Serial.printf("JSON parse error: %s\n", err.c_str());
        return;
    }

    String command = doc["command"] | "";
    Serial.printf("Command: %s -> %s\n", topic, msg.c_str());

    if (command == "switch") {
        bool on = doc["params"]["on"] | false;
        relayState = on;
        digitalWrite(RELAY_PIN, on ? HIGH : LOW);
        Serial.printf("Relay: %s\n", on ? "ON" : "OFF");
    }

    // 发送响应
    String responseTopic = String("agri/node/") + NODE_ID + "/response/cmd";
    mqtt.publish(responseTopic.c_str(), "{\"status\":\"ok\"}");
}
