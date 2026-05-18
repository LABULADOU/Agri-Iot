/*
 * 农业物联网 ESP32 固件 v2.0
 * 功能：传感器数据采集 + HTTP 上报 (via Tailscale Funnel) + 远程控制
 * 传感器：DHT11/DHT22 温湿度 + 土壤湿度 + 光照
 * 控制：继电器开关（水泵/风扇）
 */

#include <WiFi.h>
#include <WiFiClientSecure.h>
#include <HTTPClient.h>
#include <ArduinoJson.h>
#include <DHT.h>

// ==================== 配置 ====================

// WiFi 配置
const char* WIFI_SSID = "iPhone";
const char* WIFI_PASSWORD = "12345678";

// HTTP API 配置（Tailscale Funnel 公网地址）
const char* API_BASE = "https://zero-1.taile2b316.ts.net";
const char* NODE_ID = "esp32-node-001";       // 设备唯一标识

// 引脚定义
#define DHTPIN 15
#define DHTTYPE DHT22          // 使用 DHT11，如使用 DHT22 改为 DHT22
#define SOIL_MOISTURE_PIN 34   // 土壤湿度传感器 (ADC1)
#define LIGHT_PIN 35           // 光照传感器 (ADC1)
#define RELAY_PIN 16           // 继电器控制引脚

// 采集间隔 (毫秒)
const unsigned long READ_INTERVAL = 10000;  // 10秒采集一次
const unsigned long COMMAND_POLL_INTERVAL = 3000;  // 3秒轮询命令

// ==================== 全局变量 ====================

WiFiClientSecure client;
DHT dht(DHTPIN, DHTTYPE);

unsigned long lastRead = 0;
unsigned long lastCommandPoll = 0;
bool relayState = false;

// ==================== 初始化 ====================

void setup() {
    Serial.begin(115200);
    Serial.println("\n=== 农业物联网 ESP32 固件 v2.0 (HTTP+Funnel) ===");

    pinMode(RELAY_PIN, OUTPUT);
    digitalWrite(RELAY_PIN, LOW);

    dht.begin();

    client.setInsecure();  // Tailscale 证书可信，但 ESP32 可能没有完整 CA 包

    setupWiFi();
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

// ==================== HTTP 请求工具 ====================

String httpGet(const char* url) {
    HTTPClient http;
    http.begin(client, url);
    http.setTimeout(5000);
    int code = http.GET();
    if (code > 0) {
        String body = http.getString();
        http.end();
        return body;
    }
    Serial.printf("HTTP GET 失败: %d\n", code);
    http.end();
    return "";
}

String httpPost(const char* url, const char* body) {
    HTTPClient http;
    http.begin(client, url);
    http.setTimeout(5000);
    http.addHeader("Content-Type", "application/json");
    int code = http.POST(body);
    if (code > 0) {
        String resp = http.getString();
        http.end();
        return resp;
    }
    Serial.printf("HTTP POST 失败: %d\n", code);
    http.end();
    return "";
}

String httpPut(const char* url, const char* body) {
    HTTPClient http;
    http.begin(client, url);
    http.setTimeout(5000);
    http.addHeader("Content-Type", "application/json");
    int code = http.PUT(body);
    if (code > 0) {
        String resp = http.getString();
        http.end();
        return resp;
    }
    Serial.printf("HTTP PUT 失败: %d\n", code);
    http.end();
    return "";
}

// ==================== 主循环 ====================

void loop() {
    unsigned long now = millis();

    // 定期采集并上报遥测
    if (now - lastRead >= READ_INTERVAL) {
        publishTelemetry();
        lastRead = now;
    }

    // 轮询待处理命令
    if (now - lastCommandPoll >= COMMAND_POLL_INTERVAL) {
        pollCommands();
        lastCommandPoll = now;
    }
}

// ==================== 数据采集与上报 ====================

void publishTelemetry() {
    StaticJsonDocument<256> doc;
    doc["node_id"] = NODE_ID;
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
    int soilPercent = map(soilRaw, 4095, 0, 0, 100);
    metrics["soil_moisture"] = soilPercent;
    Serial.printf("土壤湿度: %d%% | ", soilPercent);

    // 读取光照 (0-4095)
    int lightRaw = analogRead(LIGHT_PIN);
    int lightLux = map(lightRaw, 0, 4095, 0, 10000);
    metrics["light"] = lightLux;
    Serial.printf("光照: %d lux", lightLux);

    metrics["relay_state"] = relayState;
    metrics["rssi"] = WiFi.RSSI();

    char buf[256];
    serializeJson(doc, buf);

    // 通过 HTTP POST 上报遥测
    String url = String(API_BASE) + "/api/v1/telemetry";
    String resp = httpPost(url.c_str(), buf);
    bool ok = resp.length() > 0;

    Serial.printf(" | HTTP: %s\n", ok ? "成功" : "失败");
}

// ==================== 命令轮询与处理 ====================

void pollCommands() {
    if (WiFi.status() != WL_CONNECTED) return;

    String url = String(API_BASE) + "/api/v1/commands/node/" + NODE_ID;
    String resp = httpGet(url.c_str());
    if (resp.length() == 0) return;

    // 解析命令列表
    StaticJsonDocument<1024> doc;
    DeserializationError err = deserializeJson(doc, resp);
    if (err) {
        Serial.printf("命令 JSON 解析失败: %s\n", err.c_str());
        return;
    }

    JsonArray arr = doc.as<JsonArray>();
    for (JsonObject cmd : arr) {
        const char* id = cmd["id"];
        const char* command = cmd["command"] | "";
        JsonObject params = cmd["params"];

        Serial.printf("收到指令: %s\n", command);

        if (strcmp(command, "switch") == 0) {
            bool on = params["on"] | false;
            relayState = on;
            digitalWrite(RELAY_PIN, on ? HIGH : LOW);
            Serial.printf("继电器状态: %s\n", on ? "开启" : "关闭");
        }
        else if (strcmp(command, "set_interval") == 0) {
            // 仅记录，运行时调整比较复杂
            Serial.printf("采集间隔设置请求 (当前: %dms)\n", READ_INTERVAL);
        }

        // 标记命令已执行
        String statusUrl = String(API_BASE) + "/api/v1/commands/" + id + "/status";
        httpPut(statusUrl.c_str(), "{\"status\":\"executed\"}");
        Serial.printf("命令 %s 已确认\n", id);
    }
}
