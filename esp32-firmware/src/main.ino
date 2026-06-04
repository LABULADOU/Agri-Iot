/*
 * 农业物联网 ESP32 固件 v3.0
 * 功能：传感器数据采集 + MQTT 上报 (LAN TCP / WAN WebSocket) + 远程控制
 * 传感器：DHT22 空气温湿度 + RS485 土壤传感器（温度/湿度/EC）
 * 控制：继电器开关（MQTT 命令订阅）
 * 
 * 依赖库:
 *   - PubSubClient (knolleary)
 *   - WebSockets (me-no-dev, 用于 WAN WebSocket path)
 *   - ArduinoJson (bblanchon)
 *   - DHT sensor library (adafruit)
 *   - ESP32 Arduino Core (内置 WiFi, LittleFS, mDNS)
 */

#include <WiFi.h>
#include <WiFiClientSecure.h>
#include <PubSubClient.h>
#include <WebSocketsClient.h>
#include <ArduinoJson.h>
#include <DHT.h>
#include <HardwareSerial.h>
#include <LittleFS.h>
#include <ESPmDNS.h>

// ==================== 配置 ====================

// WiFi 配置
const char* WIFI_SSID = "iPhone";
const char* WIFI_PASSWORD = "12345678";

// 局域网 MQTT 配置（通过 mDNS 动态解析）
#define MQTT_LAN_PORT 1883
char LAN_HOST[32] = {0};
unsigned long lastLanResolve = 0;
#define LAN_RESOLVE_INTERVAL 60000

// 公网 Funnel WebSocket 配置（Tailscale Funnel → agri-server → MQTT broker）
const char* FUNNEL_HOST = "zero-1.taile2b316.ts.net";
const int FUNNEL_PORT = 443;
const char* FUNNEL_PATH = "/mqtt";

// 节点标识
const char* NODE_ID = "esp32-node-001";
const char* MQTT_CLIENT_ID = "esp32-node-001";
const char* MQTT_USER = "";
const char* MQTT_PASS = "";

// MQTT 主题
const char* TOPIC_TELEMETRY_PREFIX = "agri/node/";
const char* TOPIC_STATUS_PREFIX = "agri/node/";
const char* TOPIC_COMMAND_PREFIX = "agri/node/";

// 引脚定义
#define DHTPIN 15
#define DHTTYPE DHT22

#define RS485_RX    16
#define RS485_TX    17
#define RS485_DIR   4
#define RELAY_PIN   2

uint8_t soilAddr = 0x01;
uint32_t soilBaud = 4800;
#define SOIL_TIMEOUT 1000

// 采集间隔
const unsigned long READ_INTERVAL = 10000;
const unsigned long MQTT_RECONNECT_INTERVAL = 5000;

// 离线缓冲区
#define BUFFER_FILE "/buffer.dat"
#define BUFFER_TMP "/buffer.tmp"
#define BUFFER_MAX_LINES 2000
#define BUFFER_FLUSH_BATCH 20
#define MQTT_BUF_SIZE 512

// ==================== 全局变量 ====================

WiFiClient lanTcp;
WiFiClientSecure wanTls;
PubSubClient mqtt(lanTcp);
WebSocketsClient webSocket;
DHT dht(DHTPIN, DHTTYPE);
HardwareSerial soilSerial(2);

unsigned long lastRead = 0;
unsigned long lastMqttReconnect = 0;
bool relayState = false;
unsigned long mqttSeq = 0;

// 连接模式枚举
enum MqttTransport {
    TRANSPORT_NONE,
    TRANSPORT_LAN_TCP,
    TRANSPORT_WAN_WS
};
MqttTransport activeTransport = TRANSPORT_NONE;
bool wanWsReady = false;
bool wanWsConnected = false;  // WebSocket-level connected
bool wanMqttConnected = false; // MQTT-level connected (over WS)

// 接收缓冲区（WebSocket path 用）
uint8_t wsRxBuf[256];
size_t wsRxLen = 0;

// ==================== Modbus CRC16 ====================

uint16_t modbusCRC16(const uint8_t* data, size_t len) {
    uint16_t crc = 0xFFFF;
    for (size_t i = 0; i < len; i++) {
        crc ^= data[i];
        for (uint8_t j = 0; j < 8; j++) {
            if (crc & 1) crc = (crc >> 1) ^ 0xA001;
            else crc >>= 1;
        }
    }
    return crc;
}

// ==================== RS485 控制 ====================

void rs485Transmit() { digitalWrite(RS485_DIR, HIGH); delayMicroseconds(10); }
void rs485Receive()  { digitalWrite(RS485_DIR, LOW);  delayMicroseconds(10); }

// ==================== 读取土壤传感器 ====================

bool readSoilSensor(float& outTemp, float& outMoist, float& outEC) {
    const int MAX_RETRIES = 2;
    for (int attempt = 0; attempt <= MAX_RETRIES; attempt++) {
        if (attempt > 0) { delay(200); Serial.printf("(重试 %d/%d) ", attempt, MAX_RETRIES); }

        outTemp = -999.0f; outMoist = -999.0f; outEC = -999.0f;

        while (soilSerial.available()) soilSerial.read();

        uint8_t req[] = { soilAddr, 0x03, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00 };
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
            while (soilSerial.available() && pos < sizeof(resp)) resp[pos++] = soilSerial.read();
            if (pos >= 11) break;
            delay(1);
        }

        if (pos == 0) {
            if (attempt == MAX_RETRIES) Serial.printf("土壤无响应");
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
            if (attempt == MAX_RETRIES) Serial.printf("土壤 CRC 失败");
        } else if (attempt == MAX_RETRIES) {
            Serial.printf("土壤格式异常 (%d字节)", pos);
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

    rs485Transmit();
    soilSerial.write(req, sizeof(req));
    soilSerial.flush();
    delay(20);
    rs485Receive();

    unsigned long start = millis();
    uint8_t resp[16] = {0};
    size_t pos = 0;
    while (millis() - start < timeout) {
        while (soilSerial.available() && pos < sizeof(resp)) resp[pos++] = soilSerial.read();
        if (pos >= 11) break;
        delay(1);
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
                soilBaud = baudRates[b]; soilAddr = a;
                Serial.printf(">> 发现传感器: 波特率 %d, 地址 0x%02X\n", soilBaud, soilAddr);
                return;
            }
            delay(1);
        }
    }
    for (uint8_t a = 0x10; a <= 0xF7; a++) {
        if (tryModbusRead(4800, a, 50)) {
            soilBaud = 4800; soilAddr = a;
            Serial.printf(">> 发现传感器 @4800: 地址 0x%02X\n", soilAddr);
            return;
        }
        delay(1);
    }
    Serial.println(">> 未找到传感器");
    diagnoseRS485();
}

void diagnoseRS485() {
    Serial.println("\n=== RS485 硬件诊断 ===");
    Serial.println("测试: UART2 回环检查");
    Serial.println("操作: 短接 GPIO17(TX2) 与 GPIO16(RX2)，然后按复位");
    for (int i = 5; i > 0; i--) { Serial.printf("  %d 秒后...\n", i); delay(1000); }

    soilSerial.begin(9600, SERIAL_8N1, RS485_RX, RS485_TX);
    delay(50);
    while (soilSerial.available()) soilSerial.read();
    soilSerial.write("U2OK\n");
    soilSerial.flush();
    delay(50);

    if (soilSerial.available()) {
        char echo[32]; size_t elen = 0;
        while (soilSerial.available() && elen < sizeof(echo)-1) echo[elen++] = soilSerial.read();
        echo[elen] = '\0';
        Serial.printf("结果: UART2 回环成功 (\"%s\") ✓\n", echo);
        Serial.println("      → ESP32 正常，问题在 TTL485 或传感器");
    } else {
        Serial.printf("结果: UART2 无回环 ✗\n");
        Serial.println("      → 检查 GPIO16/17 连接");
        Serial.println("      → 检查 TTL485 TX→GPIO16(RX), RX→GPIO17(TX)");
    }
    Serial.println("=== 诊断完成 ===");
}

// ==================== MQTT 工具函数 ====================

// 构建 MQTT topic 字符串到固定缓冲区
static void makeTopic(char* buf, size_t sz, const char* prefix, const char* suffix) {
    snprintf(buf, sz, "%s%s", prefix, suffix);
}

// 获取 telemetry topic
static void telemetryTopic(char* buf, size_t sz) {
    snprintf(buf, sz, "%s%s/telemetry", TOPIC_TELEMETRY_PREFIX, NODE_ID);
}

// 获取 status topic
static void statusTopic(char* buf, size_t sz) {
    snprintf(buf, sz, "%s%s/status", TOPIC_STATUS_PREFIX, NODE_ID);
}

// 获取 command subscribe topic（通配符）
static void commandTopic(char* buf, size_t sz) {
    snprintf(buf, sz, "%s%s/command/#", TOPIC_COMMAND_PREFIX, NODE_ID);
}

// ==================== WebSocket MQTT 直连实现 ====================

// MQTT Control Packet types
#define MQTT_CONNECT     0x10
#define MQTT_CONNACK     0x20
#define MQTT_PUBLISH     0x30
#define MQTT_PUBACK      0x40
#define MQTT_SUBSCRIBE   0x82
#define MQTT_SUBACK      0x90
#define MQTT_DISCONNECT  0xE0

// 构建并发送 MQTT CONNECT 包（通过 WebSocket）
static bool wsSendMqttConnect() {
    // Variable header: protocol name "MQTT" (4), level 4, flags, keepalive
    const char proto[] = "\x00\x04MQTT";
    uint8_t flags = 0x02; // clean session
    uint16_t keepalive = 60; // seconds
    // Payload: client ID
    size_t idLen = strlen(MQTT_CLIENT_ID);
    
    uint16_t remaining = 10 + 2 + idLen; // vhdr(10) + id_len(2) + id
    uint8_t packet[256];
    size_t pos = 0;
    
    // Fixed header
    packet[pos++] = MQTT_CONNECT;
    // Remaining length (varint)
    if (remaining < 128) {
        packet[pos++] = remaining;
    } else {
        packet[pos++] = (remaining % 128) | 0x80;
        packet[pos++] = remaining / 128;
    }
    // Variable header
    memcpy(packet + pos, proto, 6); pos += 6;
    packet[pos++] = 4; // protocol level
    packet[pos++] = flags;
    packet[pos++] = (keepalive >> 8) & 0xFF;
    packet[pos++] = keepalive & 0xFF;
    // Payload: client ID
    packet[pos++] = (idLen >> 8) & 0xFF;
    packet[pos++] = idLen & 0xFF;
    memcpy(packet + pos, MQTT_CLIENT_ID, idLen); pos += idLen;
    
    webSocket.sendBIN(packet, pos);
    return true;
}

// 构建并发送 MQTT PUBLISH 包（通过 WebSocket）
static bool wsSendMqttPublish(const char* topic, const uint8_t* payload, size_t payloadLen, uint16_t seq) {
    size_t topicLen = strlen(topic);
    uint8_t qos = 1;
    uint16_t packetId = seq & 0xFFFF;
    
    uint16_t remaining = 2 + topicLen + (qos > 0 ? 2 : 0) + payloadLen;
    uint8_t packet[512];
    size_t pos = 0;
    
    // Fixed header
    packet[pos++] = MQTT_PUBLISH | (qos << 1); // QoS 1
    if (remaining < 128) {
        packet[pos++] = remaining;
    } else {
        packet[pos++] = (remaining % 128) | 0x80;
        packet[pos++] = remaining / 128;
    }
    // Topic
    packet[pos++] = (topicLen >> 8) & 0xFF;
    packet[pos++] = topicLen & 0xFF;
    memcpy(packet + pos, topic, topicLen); pos += topicLen;
    // Packet ID (for QoS 1)
    packet[pos++] = (packetId >> 8) & 0xFF;
    packet[pos++] = packetId & 0xFF;
    // Payload
    memcpy(packet + pos, payload, payloadLen); pos += payloadLen;
    
    webSocket.sendBIN(packet, pos);
    return true;
}

// 构建并发送 MQTT SUBSCRIBE 包
static bool wsSendMqttSubscribe(const char* topicFilter, uint16_t seq) {
    size_t flen = strlen(topicFilter);

    uint16_t remaining = 2 + 2 + flen + 1; // pktId(2) + topicLen(2) + filter + qos(1)
    uint8_t packet[128];
    size_t pos = 0;

    packet[pos++] = MQTT_SUBSCRIBE;
    if (remaining < 128) {
        packet[pos++] = remaining;
    } else {
        packet[pos++] = (remaining % 128) | 0x80;
        packet[pos++] = remaining / 128;
    }
    // Packet ID
    packet[pos++] = (seq >> 8) & 0xFF;
    packet[pos++] = seq & 0xFF;
    // Topic filter
    packet[pos++] = (flen >> 8) & 0xFF;
    packet[pos++] = flen & 0xFF;
    memcpy(packet + pos, topicFilter, flen); pos += flen;
    // Requested QoS
    packet[pos++] = 1; // QoS 1

    webSocket.sendBIN(packet, pos);
    return true;
}

// 发送 MQTT DISCONNECT
static void wsSendMqttDisconnect() {
    uint8_t pkt[] = { MQTT_DISCONNECT, 0x00 };
    webSocket.sendBIN(pkt, sizeof(pkt));
}

// 解析 MQTT 剩余长度（varint）
static size_t parseMqttRemaining(const uint8_t* buf, size_t& consumed) {
    size_t value = 0;
    int multiplier = 1;
    consumed = 0;
    for (int i = 1; i < 5; i++) {
        uint8_t byte = buf[i];
        value += (byte & 0x7F) * multiplier;
        multiplier *= 128;
        consumed++;
        if (!(byte & 0x80)) break;
    }
    return value;
}

// 处理收到的 MQTT 包（来自 WebSocket）
static void handleMqttPacket(const uint8_t* data, size_t len) {
    if (len < 2) return;
    uint8_t type = data[0] & 0xF0;
    
    size_t rlConsumed = 0;
    size_t remaining = parseMqttRemaining(data, rlConsumed);
    size_t headerLen = 1 + rlConsumed;
    
    if (len < headerLen + remaining) return; // incomplete
    
    switch (type) {
        case MQTT_CONNACK: {
            if (remaining >= 2) {
                uint8_t sessionPresent = data[headerLen];
                uint8_t returnCode = data[headerLen + 1];
                if (returnCode == 0) {
                    wanMqttConnected = true;
                    Serial.println("WebSocket MQTT: 已连接 (CONNACK)");
                    // 订阅命令主题
                    char subTopic[64];
                    commandTopic(subTopic, sizeof(subTopic));
                    wsSendMqttSubscribe(subTopic, 1);
                    // 发送在线状态
                    char status[32];
                    snprintf(status, sizeof(status), "{\"status\":\"online\",\"seq\":%llu}", mqttSeq);
                    wsSendMqttPublish("agri/node/esp32-node-001/status", 
                                       (uint8_t*)status, strlen(status), ++mqttSeq);
                    // 回放缓冲区
                    flashFlushBuffer();
                } else {
                    Serial.printf("WebSocket MQTT: CONNACK 错误 %d\n", returnCode);
                }
            }
            break;
        }
        case MQTT_PUBACK: {
            // QoS 1 确认，可忽略
            break;
        }
        case MQTT_SUBACK: {
            Serial.println("WebSocket MQTT: 主题订阅成功 (SUBACK)");
            break;
        }
        case MQTT_PUBLISH: {
            // 收到命令！解析 topic 和 payload
            size_t off = headerLen;
            if (off + 2 > len) break;
            uint16_t tlen = (data[off] << 8) | data[off + 1];
            off += 2;
            if (off + tlen > len) break;
            // topic = agri/node/esp32-node-001/command/{cmd_id}
            off += tlen; // skip topic
            // Handle QoS if needed
            uint8_t qos = (data[0] & 0x06) >> 1;
            if (qos > 0) {
                if (off + 2 > len) break;
                uint16_t pktId = (data[off] << 8) | data[off + 1];
                off += 2;
                // Send PUBACK
                uint8_t ack[] = { MQTT_PUBACK, 0x02, (uint8_t)(pktId >> 8), (uint8_t)(pktId & 0xFF) };
                webSocket.sendBIN(ack, sizeof(ack));
            }
            // Parse JSON payload
            size_t payloadLen = len - off;
            if (payloadLen == 0) break;
            char json[128];
            size_t copyLen = payloadLen < sizeof(json)-1 ? payloadLen : sizeof(json)-1;
            memcpy(json, data + off, copyLen);
            json[copyLen] = '\0';
            handleMqttCommand(json);
            break;
        }
        default:
            break;
    }
}

// WebSocket 事件回调
void webSocketEvent(WStype_t type, uint8_t* payload, size_t length) {
    switch (type) {
        case WStype_DISCONNECTED:
            Serial.println("WebSocket: 断开连接");
            wanWsConnected = false;
            wanMqttConnected = false;
            break;
        case WStype_CONNECTED:
            Serial.printf("WebSocket: 已连接 (agri-server MQTT bridge)\n");
            wanWsConnected = true;
            wanMqttConnected = false;
            wsSendMqttConnect();
            break;
        case WStype_BIN:
            // 收到的二进制数据就是 MQTT 包
            handleMqttPacket(payload, length);
            break;
        case WStype_TEXT:
            // 理论上 MQTT bridge 只发 binary，忽略文本
            break;
        default:
            break;
    }
}

// ==================== MQTT LAN 回调 ====================

void mqttLanCallback(char* topic, byte* payload, unsigned int length) {
    char json[128];
    size_t copyLen = length < sizeof(json)-1 ? length : sizeof(json)-1;
    memcpy(json, payload, copyLen);
    json[copyLen] = '\0';
    handleMqttCommand(json);
}

// ==================== 命令处理（共享） ====================

void handleMqttCommand(const char* json) {
    StaticJsonDocument<192> doc;
    DeserializationError err = deserializeJson(doc, json);
    if (err) return;
    
    const char* command = doc["command"] | "";
    JsonObject params = doc["params"];
    
    Serial.printf("收到指令: %s\n", command);
    
    if (strcmp(command, "switch") == 0) {
        bool on = params["on"] | false;
        relayState = on;
        digitalWrite(RELAY_PIN, on ? HIGH : LOW);
        Serial.printf("继电器: %s\n", on ? "开启" : "关闭");
    }
    else if (strcmp(command, "set_interval") == 0) {
        Serial.printf("采集间隔调整请求 (当前: %dms)\n", READ_INTERVAL);
    }
}

// ==================== MQTT 连接管理 ====================

// 尝试 LAN TCP MQTT 连接
bool connectLanMqtt() {
    if (LAN_HOST[0] == '\0') resolveLanHost();
    
    Serial.printf("MQTT LAN: 连接 %s:%d ...\n", LAN_HOST, MQTT_LAN_PORT);
    lanTcp.stop();
    mqtt.setClient(lanTcp);
    mqtt.setServer(LAN_HOST, MQTT_LAN_PORT);
    mqtt.setCallback(mqttLanCallback);
    
    if (mqtt.connect(MQTT_CLIENT_ID, MQTT_USER, MQTT_PASS)) {
        Serial.println("MQTT LAN: 已连接");
        char subTopic[64];
        commandTopic(subTopic, sizeof(subTopic));
        mqtt.subscribe(subTopic);
        // 发布在线状态
        char status[64];
        snprintf(status, sizeof(status), "{\"status\":\"online\",\"seq\":%llu}", mqttSeq);
        char statTopic[64];
        statusTopic(statTopic, sizeof(statTopic));
        mqtt.publish(statTopic, status);
        activeTransport = TRANSPORT_LAN_TCP;
        // 回放缓冲区
        flashFlushBuffer();
        return true;
    }
    Serial.printf("MQTT LAN: 连接失败 (rc=%d)\n", mqtt.state());
    return false;
}

// 尝试 WAN WebSocket MQTT 连接
bool connectWanWsMqtt() {
    Serial.printf("WebSocket MQTT: 连接 %s:%d%s ...\n", FUNNEL_HOST, FUNNEL_PORT, FUNNEL_PATH);
    
    webSocket.beginSSL(FUNNEL_HOST, FUNNEL_PORT, FUNNEL_PATH);
    webSocket.onEvent(webSocketEvent);
    webSocket.setReconnectInterval(10000);
    
    // 等待 WebSocket 连接和 MQTT CONNACK（最多 5 秒）
    unsigned long start = millis();
    while (millis() - start < 5000) {
        webSocket.loop();
        if (wanMqttConnected) {
            activeTransport = TRANSPORT_WAN_WS;
            Serial.println("WebSocket MQTT: 连接成功");
            return true;
        }
        delay(10);
    }
    Serial.println("WebSocket MQTT: 连接超时");
    webSocket.disconnect();
    wanWsConnected = false;
    return false;
}

// 断开当前 MQTT 连接
void disconnectMqtt() {
    if (activeTransport == TRANSPORT_LAN_TCP) {
        mqtt.disconnect();
        lanTcp.stop();
    } else if (activeTransport == TRANSPORT_WAN_WS) {
        wsSendMqttDisconnect();
        webSocket.disconnect();
        wanWsConnected = false;
        wanMqttConnected = false;
    }
    activeTransport = TRANSPORT_NONE;
}

// 确保 MQTT 已连接（自动重连，优先 LAN）
void ensureMqttConnected() {
    // 检查当前连接是否存活
    if (activeTransport == TRANSPORT_LAN_TCP) {
        if (mqtt.connected()) return;
        Serial.println("MQTT LAN: 断开，尝试重连");
        activeTransport = TRANSPORT_NONE;
    } else if (activeTransport == TRANSPORT_WAN_WS) {
        webSocket.loop();
        if (wanMqttConnected) return;
        if (wanWsConnected) {
            // WebSocket 还连着但 MQTT 层断开，重新 CONNECT
            if (millis() - lastMqttReconnect > MQTT_RECONNECT_INTERVAL) {
                wsSendMqttConnect();
                lastMqttReconnect = millis();
            }
            return;
        }
        // WebSocket 也断了，需要完全重连
        activeTransport = TRANSPORT_NONE;
    }
    
    if (millis() - lastMqttReconnect < MQTT_RECONNECT_INTERVAL) return;
    lastMqttReconnect = millis();
    
    // 先尝试 LAN TCP
    if (connectLanMqtt()) return;
    
    // 再尝试 WAN WebSocket
    if (connectWanWsMqtt()) return;
    
    Serial.println("MQTT: 所有通道失败");
}

// ==================== 发布遥测 ====================

void publishMqttTelemetry(const char* jsonPayload) {
    char topic[64];
    telemetryTopic(topic, sizeof(topic));
    
    mqttSeq++;
    
    bool ok = false;
    if (activeTransport == TRANSPORT_LAN_TCP) {
        ok = mqtt.publish(topic, jsonPayload);
    } else if (activeTransport == TRANSPORT_WAN_WS) {
        ok = wsSendMqttPublish(topic, (const uint8_t*)jsonPayload, strlen(jsonPayload), (uint16_t)mqttSeq);
        // Pump WebSocket to send
        webSocket.loop();
    }
    
    if (ok) {
        Serial.print(" | MQTT: 成功 (seq=");
        Serial.print(mqttSeq);
        Serial.print(")");
        flashFlushBuffer();
    } else {
        Serial.print(" | MQTT: 发送失败");
        appendToBuffer(jsonPayload);
    }
}

void publishTelemetry() {
    if (WiFi.status() != WL_CONNECTED) return;
    ensureMqttConnected();
    if (activeTransport == TRANSPORT_NONE) return;
    
    StaticJsonDocument<384> doc;
    doc["node_id"] = NODE_ID;
    doc["seq"] = mqttSeq + 1;  // include seq for dedup
    JsonObject metrics = doc.createNestedObject("metrics");
    
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
    
    char json[384];
    size_t n = serializeJson(doc, json, sizeof(json));
    if (n >= sizeof(json)) {
        Serial.println("JSON 溢出!");
        return;
    }
    
    publishMqttTelemetry(json);
    Serial.println();
}

// ==================== 离线缓冲区 ====================

void appendToBuffer(const char* line) {
    File f = LittleFS.open(BUFFER_FILE, "a");
    if (!f) { Serial.println("缓冲区写入失败"); return; }
    f.println(line);
    f.close();
}

void trimBufferTail() {
    File rf = LittleFS.open(BUFFER_TMP, "r");
    if (!rf) return;
    int totalLines = 0;
    int c;
    while ((c = rf.read()) >= 0) { if (c == '\n') totalLines++; }
    if (totalLines <= BUFFER_MAX_LINES) {
        rf.close();
        LittleFS.rename(BUFFER_TMP, BUFFER_FILE);
        return;
    }
    int skip = totalLines - BUFFER_MAX_LINES;
    rf.seek(0);
    File wf = LittleFS.open(BUFFER_FILE, "w");
    if (!wf) { rf.close(); return; }
    char line[384];
    int lineNo = 0;
    while (rf.available()) {
        size_t len = rf.readBytesUntil('\n', line, sizeof(line)-1);
        if (len == 0) continue;
        line[len] = '\0';
        lineNo++;
        if (lineNo > skip) wf.println(line);
    }
    rf.close(); wf.close();
    LittleFS.remove(BUFFER_TMP);
}

void flashFlushBuffer() {
    if (!LittleFS.exists(BUFFER_FILE)) return;
    if (activeTransport == TRANSPORT_NONE) return;
    
    File rf = LittleFS.open(BUFFER_FILE, "r");
    if (!rf) return;
    File wf = LittleFS.open(BUFFER_TMP, "w");
    if (!wf) { rf.close(); return; }
    
    char line[384];
    int sent = 0, remaining = 0;
    
    while (rf.available()) {
        size_t len = rf.readBytesUntil('\n', line, sizeof(line)-1);
        if (len == 0) continue;
        line[len] = '\0';
        if (len > 0 && line[len-1] == '\r') line[--len] = '\0';
        
        if (sent < BUFFER_FLUSH_BATCH) {
            // Re-publish via MQTT instead of HTTP
            char topic[64];
            telemetryTopic(topic, sizeof(topic));
            bool ok = false;
            if (activeTransport == TRANSPORT_LAN_TCP) {
                ok = mqtt.publish(topic, line);
            } else if (activeTransport == TRANSPORT_WAN_WS) {
                ok = wsSendMqttPublish(topic, (uint8_t*)line, strlen(line), (uint16_t)(++mqttSeq));
                webSocket.loop();
            }
            if (ok) { sent++; continue; }
        }
        wf.println(line);
        remaining++;
    }
    
    rf.close(); wf.close();
    LittleFS.remove(BUFFER_FILE);
    if (remaining > 0) trimBufferTail();
    else LittleFS.remove(BUFFER_TMP);
    
    if (sent > 0) Serial.printf("缓冲区: %d条已发送, %d条剩余\n", sent, remaining);
}

// ==================== WiFi ====================

void resolveLanHost() {
    if (MDNS.queryHost("agri-server", LAN_HOST, sizeof(LAN_HOST))) {
        Serial.printf("mDNS: agri-server.local → %s\n", LAN_HOST);
    } else {
        strncpy(LAN_HOST, "172.20.10.6", sizeof(LAN_HOST));
        LAN_HOST[sizeof(LAN_HOST)-1] = '\0';
        Serial.printf("mDNS: agri-server.local 未找到, 使用 %s\n", LAN_HOST);
    }
    lastLanResolve = millis();
}

void setupWiFi() {
    Serial.printf("连接 WiFi: %s\n", WIFI_SSID);
    WiFi.mode(WIFI_STA);
    WiFi.begin(WIFI_SSID, WIFI_PASSWORD);
    WiFi.setAutoReconnect(true);
    
    for (int retry = 0; retry < 3; retry++) {
        int timeout = 30;
        while (WiFi.status() != WL_CONNECTED && timeout > 0) {
            delay(500); Serial.print("."); timeout--;
        }
        if (WiFi.status() == WL_CONNECTED) {
            Serial.printf("\nWiFi 已连接! IP: %s\n", WiFi.localIP().toString().c_str());
            if (MDNS.begin("esp32-node-001")) Serial.println("mDNS 就绪");
            resolveLanHost();
            return;
        }
        Serial.printf("\nWiFi 重试 %d/3...\n", retry+1);
    }
    Serial.println("\nWiFi 失败，重启");
    ESP.restart();
}

// ==================== 初始化 ====================

void setup() {
    Serial.begin(115200);
    Serial.println("\n=== 农业物联网 ESP32 固件 v3.0 (纯 MQTT) ===");
    
    pinMode(RELAY_PIN, OUTPUT);
    digitalWrite(RELAY_PIN, LOW);
    
    pinMode(RS485_DIR, OUTPUT);
    digitalWrite(RS485_DIR, LOW);
    
    soilSerial.begin(soilBaud, SERIAL_8N1, RS485_RX, RS485_TX);
    dht.begin();
    wanTls.setInsecure();
    
    scanSoilSensor();
    
    if (!LittleFS.begin()) {
        Serial.println("LittleFS 初始化失败");
    } else {
        Serial.println("LittleFS 就绪");
    }
    
    setupWiFi();
}

// ==================== 主循环 ====================

void loop() {
    unsigned long now = millis();
    
    // 保持 MQTT 连接
    if (WiFi.status() == WL_CONNECTED) {
        if (activeTransport == TRANSPORT_LAN_TCP) {
            mqtt.loop();
        } else if (activeTransport == TRANSPORT_WAN_WS) {
            webSocket.loop();
        }
        // 每 5 秒尝试重连
        if (now - lastMqttReconnect >= MQTT_RECONNECT_INTERVAL) {
            ensureMqttConnected();
        }
    }
    
    // 定期采集并上报遥测
    if (now - lastRead >= READ_INTERVAL) {
        lastRead = now;
        publishTelemetry();
    }
}
