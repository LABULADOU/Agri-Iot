/*
 * 农业物联网 ESP32 固件 v2.1
 * 功能：传感器数据采集 + HTTP 上报 (via Tailscale Funnel) + 远程控制
 * 传感器：DHT22 空气温湿度 + RS485 土壤传感器（温度/湿度/EC）
 * 控制：继电器开关（水泵/风扇）
 */

#include <WiFi.h>
#include <WiFiClientSecure.h>
#include <HTTPClient.h>
#include <ArduinoJson.h>
#include <DHT.h>
#include <HardwareSerial.h>

// ==================== 配置 ====================

// WiFi 配置
const char* WIFI_SSID = "iPhone";
const char* WIFI_PASSWORD = "12345678";

// HTTP API 配置（Tailscale Funnel 公网地址）
const char* API_BASE = "https://zero-1.taile2b316.ts.net";
const char* NODE_ID = "esp32-node-001";       // 设备唯一标识

// 引脚定义
#define DHTPIN 15
#define DHTTYPE DHT22

// RS485 (MAX485 模块 → UART2)
#define RS485_RX    16   // 接 MAX485 RO
#define RS485_TX    17   // 接 MAX485 DI
#define RS485_DIR   4    // 接 MAX485 DE+RE (高电平发送，低电平接收)
#define RELAY_PIN   2    // 继电器控制引脚

// RS485 土壤传感器 Modbus 参数（可被扫描覆盖）
uint8_t soilAddr = 0x01;           // 传感器 Modbus 地址
uint32_t soilBaud = 4800;          // 传感器波特率
#define SOIL_TIMEOUT 1000          // 响应超时 (ms)

// 采集间隔 (毫秒)
const unsigned long READ_INTERVAL = 10000;       // 10秒采集一次
const unsigned long COMMAND_POLL_INTERVAL = 3000; // 3秒轮询命令

// ==================== 全局变量 ====================

WiFiClientSecure client;
DHT dht(DHTPIN, DHTTYPE);
HardwareSerial soilSerial(2);  // UART2

unsigned long lastRead = 0;
unsigned long lastCommandPoll = 0;
bool relayState = false;

// ==================== Modbus CRC16 ====================

uint16_t modbusCRC16(const uint8_t* data, size_t len) {
    uint16_t crc = 0xFFFF;
    for (size_t i = 0; i < len; i++) {
        crc ^= data[i];
        for (uint8_t j = 0; j < 8; j++) {
            if (crc & 1) {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    return crc;
}

// ==================== RS485 控制 ====================

void rs485Transmit() {
    digitalWrite(RS485_DIR, HIGH);
    delayMicroseconds(10);
}

void rs485Receive() {
    digitalWrite(RS485_DIR, LOW);
    delayMicroseconds(10);
}

// ==================== 读取土壤传感器 (Modbus RTU) ====================

// 返回 true 表示读取成功
bool readSoilSensor(float& outTemp, float& outMoist, float& outEC) {
    const int MAX_RETRIES = 2;
    for (int attempt = 0; attempt <= MAX_RETRIES; attempt++) {
        if (attempt > 0) {
            delay(200);
            Serial.printf("(重试 %d/%d) ", attempt, MAX_RETRIES);
        }

        outTemp = -999.0f;
        outMoist = -999.0f;
        outEC = -999.0f;

        while (soilSerial.available()) soilSerial.read();

        uint8_t req[] = {
            soilAddr, 0x03, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00
        };
        uint16_t crc = modbusCRC16(req, 6);
        req[6] = crc & 0xFF;
        req[7] = (crc >> 8) & 0xFF;

        rs485Transmit();
        soilSerial.write(req, sizeof(req));
        soilSerial.flush();
        delay(20);
        rs485Receive();

        delay(50);

        unsigned long start = millis();
        uint8_t resp[16] = {0};
        size_t pos = 0;
        while (millis() - start < SOIL_TIMEOUT) {
            while (soilSerial.available() && pos < sizeof(resp)) {
                resp[pos++] = soilSerial.read();
            }
            if (pos >= 11) break;
        }

        if (pos == 0) {
            if (attempt == MAX_RETRIES) {
                Serial.printf("土壤无响应, 请求: %02X %02X %02X %02X %02X %02X %02X %02X",
                    req[0], req[1], req[2], req[3], req[4], req[5], req[6], req[7]);
            }
            continue;
        }

        if (pos >= 11 && resp[0] == soilAddr && resp[1] == 0x03 && resp[2] == 6) {
            uint16_t recvCRC = resp[9] | ((uint16_t)resp[10] << 8);
            uint16_t calcCRC = modbusCRC16(resp, 9);
            if (recvCRC == calcCRC) {
                uint16_t rawMoist = (resp[3] << 8) | resp[4];
                int16_t  rawTemp  = (int16_t)((resp[5] << 8) | resp[6]);
                uint16_t rawEC    = (resp[7] << 8) | resp[8];

                outMoist = rawMoist / 10.0f;
                outTemp  = rawTemp  / 10.0f;
                outEC    = rawEC;
                Serial.printf("土壤: 湿度=%.1f%% 温度=%.1f℃ EC=%.0fµS/cm", outMoist, outTemp, outEC);
                return true;
            }
            if (attempt == MAX_RETRIES) {
                Serial.printf("土壤 CRC 失败 (收到CRC: %02X%02X, 计算: %04X)", resp[9], resp[10], calcCRC);
            }
        } else if (attempt == MAX_RETRIES) {
            Serial.printf("土壤格式异常 (%d字节:", pos);
            for (uint8_t i = 0; i < pos; i++) Serial.printf(" %02X", resp[i]);
            Serial.print(")");
        }
    }

    return false;
}

// ==================== Modbus 扫描诊断 ====================

bool tryModbusRead(uint32_t baud, uint8_t addr, unsigned long timeout) {
    soilSerial.begin(baud, SERIAL_8N1, RS485_RX, RS485_TX);
    delay(10);
    while (soilSerial.available()) soilSerial.read();

    uint8_t req[] = { addr, 0x03, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00 };
    uint16_t crc = modbusCRC16(req, 6);
    req[6] = crc & 0xFF;
    req[7] = (crc >> 8) & 0xFF;

    soilSerial.write(req, sizeof(req));
    soilSerial.flush();
    delay(20);

    unsigned long start = millis();
    uint8_t resp[16] = {0};
    size_t pos = 0;
    while (millis() - start < timeout) {
        while (soilSerial.available() && pos < sizeof(resp)) {
            resp[pos++] = soilSerial.read();
        }
        if (pos >= 11) break;
    }

    if (pos >= 11 && resp[0] == addr && resp[1] == 0x03 && resp[2] == 6) {
        uint16_t recvCRC = resp[9] | ((uint16_t)resp[10] << 8);
        uint16_t calcCRC = modbusCRC16(resp, 9);
        return recvCRC == calcCRC;
    }
    return false;
}

void scanSoilSensor() {
    uint32_t baudRates[] = {2400, 4800, 9600, 19200, 38400, 115200};
    const int numBauds = 6;

    Serial.println("\n=== 扫描土壤传感器 ===");

    if (tryModbusRead(soilBaud, soilAddr, 200)) {
        Serial.printf(">> 当前配置有效: 波特率 %d, 地址 0x%02X\n", soilBaud, soilAddr);
        return;
    }

    for (int b = 0; b < numBauds; b++) {
        for (uint8_t a = 0x01; a <= 0x0F; a++) {
            if (baudRates[b] == soilBaud && a == soilAddr) continue;
            if (tryModbusRead(baudRates[b], a, 80)) {
                soilBaud = baudRates[b];
                soilAddr = a;
                Serial.printf(">> 发现传感器: 波特率 %d, 地址 0x%02X\n", soilBaud, soilAddr);
                return;
            }
        }
    }

    for (uint8_t a = 0x10; a <= 0xF7; a++) {
        if (tryModbusRead(4800, a, 50)) {
            soilBaud = 4800;
            soilAddr = a;
            Serial.printf(">> 发现传感器 @4800: 地址 0x%02X\n", soilAddr);
            return;
        }
    }

    Serial.println(">> 未找到传感器，启动硬件诊断...");
    diagnoseRS485();
}

void diagnoseRS485() {
    Serial.println("\n=== RS485 硬件诊断 ===");

    // 测试 UART2 本身（需要短接 GPIO17→GPIO16）
    Serial.println("测试: UART2 回环检查");
    Serial.println("操作: 用杜邦线短接 GPIO17(TX2) 与 GPIO16(RX2)，然后按复位");
    for (int i = 5; i > 0; i--) {
        Serial.printf("  %d 秒后开始...\n", i);
        delay(1000);
    }

    soilSerial.begin(9600, SERIAL_8N1, RS485_RX, RS485_TX);
    delay(50);
    while (soilSerial.available()) soilSerial.read();

    soilSerial.write("U2OK\n");
    soilSerial.flush();
    delay(50);

    if (soilSerial.available()) {
        String echo = soilSerial.readString();
        Serial.printf("结果: UART2 回环成功 (收到 \"%s\") ✓\n", echo.c_str());
        Serial.println("      → ESP32 侧正常，问题在 TTL485 模块或传感器");
        Serial.println("解决: 换一个 TTL485 模块试试 (淘宝几块钱)");
    } else {
        Serial.printf("结果: UART2 无回环 ✗\n");
        Serial.println("      → 检查 GPIO16/17 是否虚焊或接错");
        Serial.println("      → 检查 TTL485 模块 TX/RX 是否接反");
        Serial.println("      → TTL485 的 TX → ESP32 GPIO16(RX)");
        Serial.println("      → TTL485 的 RX → ESP32 GPIO17(TX)");
    }

    Serial.println("=== 诊断完成 ===");
}

// ==================== 初始化 ====================

void setup() {
    Serial.begin(115200);
    Serial.println("\n=== 农业物联网 ESP32 固件 v2.1 (RS485 土壤传感器) ===");

    // 继电器
    pinMode(RELAY_PIN, OUTPUT);
    digitalWrite(RELAY_PIN, LOW);

    // RS485 方向控制
    pinMode(RS485_DIR, OUTPUT);
    digitalWrite(RS485_DIR, LOW);  // 默认接收

    // 初始化 UART2
    soilSerial.begin(soilBaud, SERIAL_8N1, RS485_RX, RS485_TX);

    dht.begin();
    client.setInsecure();

    // 启动后先扫描传感器配置
    scanSoilSensor();

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
    StaticJsonDocument<384> doc;
    doc["node_id"] = NODE_ID;
    JsonObject metrics = doc.createNestedObject("metrics");

    // 读取 DHT22 空气温湿度
    float airTemp = dht.readTemperature();
    float airHum = dht.readHumidity();

    if (!isnan(airTemp)) {
        metrics["air_temp"] = roundf(airTemp * 100.0f) / 100.0f;
        Serial.printf("气温: %.1f℃ | ", airTemp);
    }
    if (!isnan(airHum)) {
        metrics["air_humidity"] = roundf(airHum * 100.0f) / 100.0f;
        Serial.printf("气湿: %.1f%% | ", airHum);
    }

    // 读取 RS485 土壤传感器（温度/湿度/EC）
    float soilTemp, soilMoist, soilEC;
    bool soilOk = readSoilSensor(soilTemp, soilMoist, soilEC);
    Serial.print(" | ");
    if (soilOk) {
        metrics["soil_temp"] = roundf(soilTemp * 100.0f) / 100.0f;
        metrics["soil_moisture"] = roundf(soilMoist * 100.0f) / 100.0f;
        metrics["ec"] = roundf(soilEC * 100.0f) / 100.0f;
    } else {
        Serial.print("土壤传感器异常");
    }

    metrics["relay_state"] = relayState;
    metrics["rssi"] = WiFi.RSSI();

    char buf[384];
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
        httpPut(statusUrl.c_str(), "{\"status\":\"completed\"}");
        Serial.printf("命令 %s 已确认\n", id);
    }
}
