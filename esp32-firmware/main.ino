/*
 * 农业物联网 ESP32 固件 v1.0
 * 功能：传感器数据采集 + MQTT 上报 + 远程控制
 * 传感器：DHT11/DHT22 温湿度 + 土壤湿度 + 光照
 * 控制：继电器开关（水泵/风扇）
 */

#include <WiFi.h>
#include <PubSubClient.h>
#include <ArduinoJson.h>
#include <DHT.h>

// ==================== 配置 ====================

// WiFi 配置
const char* WIFI_SSID = "ChinaNet-Jj7u";
const char* WIFI_PASSWORD = "bp9tu6fm";

// MQTT 配置
const char* MQTT_SERVER = "192.168.1.6";     // 本机局域网 IP
const int MQTT_PORT = 1883;
const char* NODE_ID = "esp32-node-001";      // 设备唯一标识

// 引脚定义
#define DHTPIN 4
#define DHTTYPE DHT11          // 使用 DHT11，如使用 DHT22 改为 DHT22
#define SOIL_MOISTURE_PIN 34   // 土壤湿度传感器 (ADC1)
#define LIGHT_PIN 35           // 光照传感器 (ADC1)
#define RELAY_PIN 16           // 继电器控制引脚

// 采集间隔 (毫秒)
const unsigned long READ_INTERVAL = 10000;  // 10秒采集一次

// ==================== 全局变量 ====================

WiFiClient espClient;
PubSubClient mqtt(espClient);
DHT dht(DHTPIN, DHTTYPE);

unsigned long lastRead = 0;
bool relayState = false;

// ==================== 初始化 ====================

void setup() {
    Serial.begin(115200);
    Serial.println("\n=== 农业物联网 ESP32 固件 v1.0 ===");
    
    pinMode(RELAY_PIN, OUTPUT);
    digitalWrite(RELAY_PIN, LOW);
    
    dht.begin();
    
    setupWiFi();
    setupMQTT();
}

// ==================== WiFi ====================

void setupWiFi() {
    Serial.printf("连接 WiFi: %s\n", WIFI_SSID);
    WiFi.begin(WIFI_SSID, WIFI_PASSWORD);
    WiFi.setAutoReconnect(true);
    
    int timeout = 20;
    while (WiFi.status() != WL_CONNECTED && timeout > 0) {
        delay(500);
        Serial.print(".");
        timeout--;
    }
    
    if (WiFi.status() == WL_CONNECTED) {
        Serial.printf("\nWiFi 已连接! IP: %s\n", WiFi.localIP().toString().c_str());
    } else {
        Serial.println("\nWiFi 连接失败，进入深度休眠...");
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
        Serial.print("连接 MQTT Broker...");
        
        String willTopic = String("agri/node/") + NODE_ID + "/status";
        
        if (mqtt.connect(NODE_ID, willTopic.c_str(), 1, true, "offline")) {
            Serial.println(" 已连接");
            
            // 发布上线状态
            mqtt.publish(willTopic.c_str(), "online");
            
            // 订阅控制指令
            String cmdTopic = String("agri/node/") + NODE_ID + "/command/#";
            mqtt.subscribe(cmdTopic.c_str());
            Serial.printf(" 已订阅: %s\n", cmdTopic.c_str());
        } else {
            Serial.printf(" 失败 (rc=%d), 5秒后重试...\n", mqtt.state());
            delay(5000);
        }
    }
}

// ==================== 主循环 ====================

void loop() {
    // 保持 MQTT 连接
    if (!mqtt.connected()) {
        reconnectMQTT();
    }
    mqtt.loop();
    
    // 定期采集数据
    unsigned long now = millis();
    if (now - lastRead >= READ_INTERVAL) {
        publishTelemetry();
        lastRead = now;
    }
}

// ==================== 数据采集与上报 ====================

void publishTelemetry() {
    StaticJsonDocument<256> doc;
    JsonObject metrics = doc.createNestedObject("metrics");
    
    // 读取 DHT 传感器
    float temp = dht.readTemperature();
    float hum = dht.readHumidity();
    
    if (!isnan(temp)) {
        metrics["temperature"] = round(temp * 100) / 100.0;
        Serial.printf("温度: %.1f℃ | ", temp);
    }
    if (!isnan(hum)) {
        metrics["humidity"] = round(hum * 100) / 100.0;
        Serial.printf("湿度: %.1f%% | ", hum);
    }
    
    // 读取土壤湿度 (0-4095)
    int soilRaw = analogRead(SOIL_MOISTURE_PIN);
    int soilPercent = map(soilRaw, 4095, 0, 0, 100);  // 反转：值越小越湿
    metrics["soil_moisture"] = soilPercent;
    Serial.printf("土壤湿度: %d%% | ", soilPercent);
    
    // 读取光照 (0-4095)
    int lightRaw = analogRead(LIGHT_PIN);
    int lightLux = map(lightRaw, 0, 4095, 0, 10000);  // 映射到 lux
    metrics["light"] = lightLux;
    Serial.printf("光照: %d lux", lightLux);
    
    // 上报状态
    metrics["relay_state"] = relayState;
    metrics["rssi"] = WiFi.RSSI();
    
    // 序列化为 JSON
    char buf[256];
    serializeJson(doc, buf);
    
    // 发布到 MQTT
    String topic = String("agri/node/") + NODE_ID + "/telemetry";
    bool ok = mqtt.publish(topic.c_str(), buf);
    
    Serial.printf(" | 上报: %s\n", ok ? "成功" : "失败");
}

// ==================== 控制指令处理 ====================

void onCommand(char* topic, byte* payload, unsigned int length) {
    // 解析消息
    String msg;
    for (unsigned int i = 0; i < length; i++) {
        msg += (char)payload[i];
    }
    
    StaticJsonDocument<128> doc;
    DeserializationError err = deserializeJson(doc, msg);
    if (err) {
        Serial.printf("JSON 解析失败: %s\n", err.c_str());
        return;
    }
    
    String command = doc["command"] | "";
    Serial.printf("收到指令: %s -> %s\n", topic, msg.c_str());
    
    if (command == "switch") {
        bool on = doc["params"]["on"] | false;
        relayState = on;
        digitalWrite(RELAY_PIN, on ? HIGH : LOW);
        Serial.printf("继电器状态: %s\n", on ? "开启" : "关闭");
    }
    else if (command == "set_interval") {
        int interval = doc["params"]["interval"] | 10;
        Serial.printf("采集间隔设置为: %d秒\n", interval);
    }
    
    // 发送执行响应
    String responseTopic = String("agri/node/") + NODE_ID + "/response/cmd";
    mqtt.publish(responseTopic.c_str(), "{\"status\":\"ok\"}");
}
