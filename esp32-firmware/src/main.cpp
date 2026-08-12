/*
 * 农业物联网 ESP32 固件 v4.0 (星型组网 + OTA)
 * 功能：传感器数据采集 + MQTT 上报 (LAN TCP / WAN WebSocket) + OTA 升级
 * 传感器：DHT22 空气温湿度 + RS485 土壤传感器（温度/湿度/EC）
 * 控制：继电器开关（MQTT 命令订阅）
 *
 * 构建: pio run -e esp32-node-001  或  pio run -e esp32-node-002
 * 需要 -DNODE_ID_STR=\"esp32-node-00x\" 编译标志
 *
 * 依赖库:
 *   - PubSubClient (knolleary)
 *   - WebSockets (me-no-dev, 用于 WAN WebSocket path)
 *   - ArduinoJson (bblanchon)
 *   - DHT sensor library (adafruit)
 *   - ESP32 Arduino Core (内置 WiFi, LittleFS, mDNS, mbedtls)
 */

#include <WiFi.h>
#include <WiFiClientSecure.h>
#include <PubSubClient.h>
#define WEBSOCKETS_TCP_TIMEOUT 15000
#include <WebSocketsClient.h>
#include <ArduinoJson.h>
#include <DHT.h>
#include <HardwareSerial.h>
#include <LittleFS.h>
#include <ESPmDNS.h>
#include <HTTPClient.h>
#include <Update.h>
#include <mbedtls/sha256.h>
#include <mbedtls/pk.h>
#include <mbedtls/base64.h>
#include <time.h>
#include "ota_public.h"

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
const char* FUNNEL_HOST = "debian.taile2b316.ts.net";
const int FUNNEL_PORT = 443;
const char* FUNNEL_PATH = "/mqtt";

// 节点标识（从编译标志 -DNODE_ID_STR=... 注入）
#ifndef NODE_ID_STR
#error "NODE_ID_STR must be defined via -DNODE_ID_STR=\"esp32-node-00x\""
#endif
const char* NODE_ID = NODE_ID_STR;
const char* FW_VERSION = "4.0.3";
const char* MQTT_CLIENT_ID = NODE_ID;
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
const unsigned long READ_INTERVAL = 300000;  // 5 分钟
const unsigned long MQTT_RECONNECT_INTERVAL = 5000;
const unsigned long TELEMETRY_SILENT_TIMEOUT = 600000;  // 10 分钟（需 > READ_INTERVAL）

// 离线缓冲区
#define BUFFER_FILE "/buffer.dat"
#define BUFFER_TMP "/buffer.tmp"
// 256KB SPIFFS: 每行 ~300B, 800 行 ≈ 240KB (留 ~16KB 给临时文件)
// 断线后采集间隔逐步降低(10s→60s→300s)，有效覆盖时间远超 2 小时
#define BUFFER_MAX_LINES 800
#define BUFFER_FLUSH_BATCH 20
#define MQTT_BUF_SIZE 5120

// ==================== 全局变量 ====================

WiFiClient lanTcp;
WiFiClientSecure wanTls;
PubSubClient mqtt(lanTcp);
WebSocketsClient webSocket;
DHT dht(DHTPIN, DHTTYPE);
HardwareSerial soilSerial(2);

unsigned long lastRead = 0;
unsigned long lastMqttReconnect = 0;
char bootId[12] = {0};
bool relayState = false;
wl_status_t lastWifiStatus = WL_CONNECTED;  // loop() 启动时 WiFi 已连接
unsigned long lastWifiOnline = 0;           // 上次 WiFi 在线的时间戳
unsigned long mqttSeq = 0;
unsigned long lastTelemetryOk = 0;      // 上次遥测发布成功的时间戳（看门狗用）
const char* lastCmdResult = "";  // OTA/命令执行结果反馈

// MQTT 流式 OTA 状态（通过现有 WebSocket MQTT 连接接收固件块）
bool otaMqttMode = false;
size_t otaMqttTotal = 0;
size_t otaMqttReceived = 0;
char otaMqttSig[128] = {0};
mbedtls_sha256_context otaMqttCtx;
bool otaMqttCtxInit = false;
int otaMqttLastPct = -1;

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

uint8_t dhtFailCount = 0;      // DHT22 consecutive failure counter
#define DHT_FAIL_THRESHOLD 3   // after 3 consecutive failures, report dht_status="failed"

// 接收缓冲区（WebSocket path 用）
uint8_t wsRxBuf[256];
size_t wsRxLen = 0;

// ==================== 前向声明 ====================

bool readSoilSensor(float& outTemp, float& outMoist, float& outEC);
bool tryModbusRead(uint32_t baud, uint8_t addr, unsigned long timeout);
void scanSoilSensor();
void diagnoseRS485();
void rs485Transmit();
void rs485Receive();
void flashFlushBuffer();
void handleMqttCommand(const char* json);
void resolveLanHost();
void appendToBuffer(const char* line);


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

static void telemetryTopic(char* buf, size_t sz) {
    snprintf(buf, sz, "%s%s/telemetry", TOPIC_TELEMETRY_PREFIX, NODE_ID);
}

static void statusTopic(char* buf, size_t sz) {
    snprintf(buf, sz, "%s%s/status", TOPIC_STATUS_PREFIX, NODE_ID);
}

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
    
    // 边界检查：防止栈缓冲区溢出
    size_t totalNeeded = 4 + topicLen + 2 + payloadLen;  // header(4 max) + topic + pktId + payload
    if (totalNeeded > 512) {
        Serial.printf("MQTT PUBLISH: packet too large (%u bytes)\n", (unsigned)totalNeeded);
        return false;
    }
    
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
    
    return webSocket.sendBIN(packet, pos);
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
static size_t parseMqttRemaining(const uint8_t* buf, size_t len, size_t& consumed) {
    if (len < 2) { consumed = 0; return 0; }
    size_t value = 0;
    int multiplier = 1;
    consumed = 0;
    for (int i = 1; i < 5 && (size_t)i < len; i++) {
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
    size_t remaining = parseMqttRemaining(data, len, rlConsumed);
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
                    char statTopic[64];
                    snprintf(statTopic, sizeof(statTopic), "agri/node/%s/status", NODE_ID);
                    wsSendMqttPublish(statTopic, (uint8_t*)status, strlen(status), ++mqttSeq);
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
            // 提取 topic
            size_t off = headerLen;
            if (off + 2 > len) break;
            uint16_t tlen = (data[off] << 8) | data[off + 1];
            off += 2;
            if (off + tlen > len) break;
            char topic[96];
            size_t topicCpLen = tlen < sizeof(topic)-1 ? tlen : sizeof(topic)-1;
            memcpy(topic, data + off, topicCpLen);
            topic[topicCpLen] = '\0';
            off += tlen;

            // OTA MQTT 固件块：二进制 payload 直接写入 Flash
            if (strstr(topic, "ota_chunk")) {
                size_t poff = off;
                uint8_t qos = (data[0] & 0x06) >> 1;
                if (qos > 0) {
                    if (poff + 2 > len) break;
                    uint16_t pktId = (data[poff] << 8) | data[poff + 1];
                    poff += 2;
                    uint8_t ack[] = { MQTT_PUBACK, 0x02, (uint8_t)(pktId >> 8), (uint8_t)(pktId & 0xFF) };
                    webSocket.sendBIN(ack, sizeof(ack));
                }
                size_t payloadLen = len - poff;
                if (!otaMqttMode) {
                    Serial.print("Z");
                    break;
                }
                // 双通道防重复：LAN TCP 通时 WebSocket 路径跳过 OTA chunk，防止 SHA256 被更新两次
                if (mqtt.connected()) {
                    break;
                }
                Serial.print("+");  // 确认 chunk handler 执行
                // SHA256 更新
                mbedtls_sha256_update(&otaMqttCtx, data + poff, payloadLen);
                // 写入 Flash (const cast needed — Update.write takes non-const uint8_t*)
                size_t written = Update.write(const_cast<uint8_t*>(data + poff), payloadLen);
                if (written != payloadLen) {
                    Serial.printf("!WF(%d/%d)", written, payloadLen);
                    lastCmdResult = "ota:write_fail";
                    Update.abort();
                    if (otaMqttCtxInit) { mbedtls_sha256_free(&otaMqttCtx); otaMqttCtxInit = false; }
                    otaMqttMode = false;
                    break;
                }
                Serial.print(".");
                otaMqttReceived += written;
                int pct = (int)((long long)otaMqttReceived * 100 / otaMqttTotal);
                int newPct = (pct / 10) * 10;
                if (newPct != otaMqttLastPct && newPct % 10 == 0 && newPct > 0) {
                    Serial.print("OTA:"); Serial.print(newPct); Serial.print("% ");
                    otaMqttLastPct = newPct;
                }
                // 最后一块：验证签名 + 完成 OTA
                if (otaMqttReceived >= otaMqttTotal) {
                    Serial.println("OTA MQTT: 接收完成，验证签名...");
                    uint8_t hash[32];
                    mbedtls_sha256_finish(&otaMqttCtx, hash);
                    mbedtls_sha256_free(&otaMqttCtx);
                    otaMqttCtxInit = false;
                    mbedtls_pk_context pk;
                    mbedtls_pk_init(&pk);
                    int ret = mbedtls_pk_parse_public_key(&pk, ota_public_der, ota_public_der_len);
                    if (ret != 0) {
                        lastCmdResult = "ota:pubkey_fail";
                        Serial.printf("OTA: 解析公钥失败 %d\n", ret);
                        Update.abort(); mbedtls_pk_free(&pk); otaMqttMode = false; break;
                    }
                    size_t sigLen;
                    uint8_t sigBuf[128];
                    ret = mbedtls_base64_decode(sigBuf, sizeof(sigBuf), &sigLen,
                        (const unsigned char*)otaMqttSig, strlen(otaMqttSig));
                    if (ret != 0) {
                        lastCmdResult = "ota:b64_fail";
                        Serial.printf("OTA: base64 解码失败 %d\n", ret);
                        mbedtls_pk_free(&pk); Update.abort(); otaMqttMode = false; break;
                    }
                    ret = mbedtls_pk_verify(&pk, MBEDTLS_MD_SHA256, hash, 32, sigBuf, sigLen);
                    mbedtls_pk_free(&pk);
                    if (ret != 0) {
                        lastCmdResult = "ota:sign_fail";
                        Serial.printf("OTA: 签名验证失败 %d\n", ret);
                        Update.abort(); otaMqttMode = false; break;
                    }
                    Serial.println("OTA: 签名验证通过");
                    if (!Update.end(true)) {
                        lastCmdResult = "ota:end_fail";
                        Serial.printf("OTA: Update.end 失败 (error=%d)\n", Update.getError());
                        otaMqttMode = false; break;
                    }
                    Serial.println("OTA: 成功，即将重启");
                    lastCmdResult = "ota:rebooting";
                    ESP.restart();
                }
                break;
            }

            // 普通 JSON 指令
            uint8_t qos = (data[0] & 0x06) >> 1;
            if (qos > 0) {
                if (off + 2 > len) break;
                uint16_t pktId = (data[off] << 8) | data[off + 1];
                off += 2;
                uint8_t ack[] = { MQTT_PUBACK, 0x02, (uint8_t)(pktId >> 8), (uint8_t)(pktId & 0xFF) };
                webSocket.sendBIN(ack, sizeof(ack));
            }
            size_t payloadLen = len - off;
            if (payloadLen == 0) break;
            char json[768];
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
            if (otaMqttMode) {
                Serial.print("!DISC");
                Update.abort();
                otaMqttMode = false;
                if (otaMqttCtxInit) {
                    mbedtls_sha256_free(&otaMqttCtx);
                    otaMqttCtxInit = false;
                }
            }
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
    // OTA MQTT 固件块：二进制 payload 直接写入 Flash (LAN TCP path)
    if (strstr(topic, "ota_chunk")) {
        if (!otaMqttMode) { Serial.print("Z"); return; }
        Serial.print("+");
        mbedtls_sha256_update(&otaMqttCtx, payload, length);
        size_t written = Update.write(payload, length);
        if (written != length) {
            Serial.printf("!WF(%d/%d)", written, length);
            lastCmdResult = "ota:write_fail";
            Update.abort();
            if (otaMqttCtxInit) { mbedtls_sha256_free(&otaMqttCtx); otaMqttCtxInit = false; }
            otaMqttMode = false;
            return;
        }
        Serial.print(".");
        otaMqttReceived += written;
        int pct = (int)((long long)otaMqttReceived * 100 / otaMqttTotal);
        int newPct = (pct / 10) * 10;
        if (newPct != otaMqttLastPct && newPct % 10 == 0 && newPct > 0) {
            Serial.print("OTA:"); Serial.print(newPct); Serial.print("% ");
            otaMqttLastPct = newPct;
        }
        if (otaMqttReceived >= otaMqttTotal) {
            Serial.println("OTA MQTT: 接收完成，验证签名...");
            uint8_t hash[32];
            mbedtls_sha256_finish(&otaMqttCtx, hash);
            mbedtls_sha256_free(&otaMqttCtx);
            otaMqttCtxInit = false;
            mbedtls_pk_context pk;
            mbedtls_pk_init(&pk);
            int ret = mbedtls_pk_parse_public_key(&pk, ota_public_der, ota_public_der_len);
            if (ret != 0) {
                lastCmdResult = "ota:pubkey_fail";
                Serial.printf("OTA: 解析公钥失败 %d\n", ret);
                Update.abort(); mbedtls_pk_free(&pk); otaMqttMode = false; return;
            }
            size_t sigLen;
            uint8_t sigBuf[128];
            ret = mbedtls_base64_decode(sigBuf, sizeof(sigBuf), &sigLen,
                (const unsigned char*)otaMqttSig, strlen(otaMqttSig));
            if (ret != 0) {
                lastCmdResult = "ota:b64_fail";
                Serial.printf("OTA: base64 解码失败 %d\n", ret);
                mbedtls_pk_free(&pk); Update.abort(); otaMqttMode = false; return;
            }
            ret = mbedtls_pk_verify(&pk, MBEDTLS_MD_SHA256, hash, 32, sigBuf, sigLen);
            mbedtls_pk_free(&pk);
            if (ret != 0) {
                lastCmdResult = "ota:sign_fail";
                Serial.printf("OTA: 签名验证失败 %d\n", ret);
                Update.abort(); otaMqttMode = false; return;
            }
            Serial.println("OTA: 签名验证通过");
            if (!Update.end(true)) {
                lastCmdResult = "ota:end_fail";
                Serial.printf("OTA: Update.end 失败 (error=%d)\n", Update.getError());
                otaMqttMode = false; return;
            }
            Serial.println("OTA: 成功，即将重启");
            lastCmdResult = "ota:rebooting";
            ESP.restart();
        }
        return;
    }

    // 普通 JSON 指令
    Serial.printf("收到 MQTT 包: topic=%s, len=%d\n", topic, length);
    char json[768];
    size_t copyLen = length < sizeof(json)-1 ? length : sizeof(json)-1;
    memcpy(json, payload, copyLen);
    json[copyLen] = '\0';
    handleMqttCommand(json);
}

// ==================== 命令处理（共享） ====================

void handleMqttCommand(const char* json) {
    StaticJsonDocument<384> doc;
    DeserializationError err = deserializeJson(doc, json);
    if (err) {
        Serial.printf("指令 JSON 解析失败: %s, json=%s\n", err.c_str(), json);
        return;
    }
    
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
    else if (strcmp(command, "ota_mqtt") == 0) {
        // MQTT 流式 OTA：固件通过现有 MQTT 连接分块传输，绕过 Funnel TLS 限制
        otaMqttTotal = params["total_size"] | 0;
        const char* sig = params["sig"] | "";
        if (otaMqttTotal == 0 || strlen(sig) == 0) {
            lastCmdResult = "ota:missing_params";
            Serial.println("OTA MQTT: 缺少 total_size 或 sig");
            return;
        }
        lastCmdResult = "ota:mqtt_started";
        strncpy(otaMqttSig, sig, sizeof(otaMqttSig) - 1);
        otaMqttSig[sizeof(otaMqttSig) - 1] = '\0';
        otaMqttReceived = 0;
        mbedtls_sha256_init(&otaMqttCtx);
        mbedtls_sha256_starts(&otaMqttCtx, 0);
        otaMqttCtxInit = true;
        if (!Update.begin(otaMqttTotal, U_FLASH)) {
            int updateErr = Update.getError();
            Serial.printf("OTA MQTT: Update.begin 失败 (error=%d)\n", updateErr);
            static char errBuf[32];
            snprintf(errBuf, sizeof(errBuf), "ota:beg_fail_%d", updateErr);
            lastCmdResult = errBuf;
            otaMqttMode = false;
            if (otaMqttCtxInit) { mbedtls_sha256_free(&otaMqttCtx); otaMqttCtxInit = false; }
            return;
        }
        otaMqttMode = true;
        otaMqttLastPct = -1;
        Serial.printf("OTA MQTT: 开始接收 %d bytes\n", otaMqttTotal);
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
        if (mqtt.subscribe(subTopic)) {
            Serial.printf("MQTT LAN: 订阅成功 %s\n", subTopic);
        } else {
            Serial.printf("MQTT LAN: 订阅失败 %s\n", subTopic);
        }
        // 发布在线状态
        char status[64];
        snprintf(status, sizeof(status), "{\"status\":\"online\",\"seq\":%llu}", mqttSeq);
        char statTopic[64];
        statusTopic(statTopic, sizeof(statTopic));
        mqtt.publish(statTopic, status);
        activeTransport = TRANSPORT_LAN_TCP;
        // 不在连接建立时 flush 缓冲区 —— 连续 20 条 publish 会淹没 TCP 连接。
        // 缓冲区将在下次 publishTelemetry() 成功时自然 flush。
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
    webSocket.setReconnectInterval(2000);
    
    // 等待 WebSocket 连接和 MQTT CONNACK（最多 30 秒，TLS 协商可能较慢）
    unsigned long start = millis();
    while (millis() - start < 30000) {
        webSocket.loop();
        if (wanMqttConnected) {
            activeTransport = TRANSPORT_WAN_WS;
            Serial.println("WebSocket MQTT: 连接成功");
            // 回放缓冲区（同 connectLanMqtt）
            flashFlushBuffer();
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

// 尝试 WiFi 重连（非阻塞，带指数退避）
// 阶段 | 间隔 | 累计时长
// 快速 |  10s | 0~1 分钟（6 次）
// 中速 |  60s | 1~5 分钟（4 次）
// 慢速 | 300s | 5~35 分钟（6 次）
// 保守 | 600s | 35 分钟后
void ensureWiFi() {
    static int failCount = 0;
    static unsigned long lastAttempt = 0;

    if (WiFi.status() == WL_CONNECTED) {
        failCount = 0;  // 重连成功，重置计数
        return;
    }

    unsigned long now = millis();
    unsigned long interval;
    if (failCount < 6)          interval = 10000;
    else if (failCount < 10)    interval = 60000;
    else if (failCount < 16)    interval = 300000;
    else                        interval = 600000;

    if (now - lastAttempt < interval) return;
    lastAttempt = now;
    failCount++;

    Serial.printf("[WiFi] 断线(%d), 尝试 %d, 间隔 %lus\n",
                  WiFi.status(), failCount, interval / 1000);
    WiFi.reconnect();
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
        // 先刷新 MQTT 状态，确保 connected() 准确
        mqtt.loop();
        ok = mqtt.publish(topic, jsonPayload);
    } else if (activeTransport == TRANSPORT_WAN_WS) {
        ok = wsSendMqttPublish(topic, (const uint8_t*)jsonPayload, strlen(jsonPayload), (uint16_t)mqttSeq);
        // Pump WebSocket to send
        webSocket.loop();
    }

    if (ok) {
        lastTelemetryOk = millis();
        Serial.print(" | MQTT: 成功 (seq=");
        Serial.print(mqttSeq);
        Serial.print(")");
        flashFlushBuffer();
    } else {
        Serial.print(" | MQTT: 发送失败");
        // 不在这里重置 lastMqttReconnect —— 那会导致 ensureMqttConnected()
        // 在每个 loop() 迭代中触发（now-0>=5000 永远为 true），
        // 连接刚建好就被 lanTcp.stop() 关掉，产生30ms重连风暴。
        // 让正常间隔逻辑处理重连。
        if (activeTransport == TRANSPORT_LAN_TCP && !mqtt.connected()) {
            Serial.print("，连接已断开，等待重连...");
            activeTransport = TRANSPORT_NONE;
        }
        // 写缓冲区
        appendToBuffer(jsonPayload);
    }
}

void publishTelemetry() {
    // 无论 WiFi 状态如何，都采集传感器数据；WiFi 离线时数据缓存到 LittleFS

    // Do NOT call ensureMqttConnected() here — TLS handshake on the 10s
    // sensor-read path blows the loopTask stack (4KB default).
    // Connection is maintained by loop() every 5s.
    // 但 mqtt.loop() 是安全的——仅刷新状态，不发起握手
    if (activeTransport == TRANSPORT_LAN_TCP) {
        mqtt.loop();
        if (!mqtt.connected()) {
            activeTransport = TRANSPORT_NONE;
        }
    } else if (activeTransport == TRANSPORT_WAN_WS) {
        webSocket.loop();
        if (!wanMqttConnected) {
            activeTransport = TRANSPORT_NONE;
        }
    }
    
    StaticJsonDocument<448> doc;
    doc["node_id"] = NODE_ID;
    doc["boot_id"] = bootId;
    doc["seq"] = mqttSeq + 1;
    doc["fw_version"] = FW_VERSION;
    {
        time_t t = time(nullptr);
        if (t > 1700000000) {  // 2023-11-14 之后的时间戳才有效
            doc["captured_at"] = (long long)t;
        }
    }
    doc["ota_status"] = "idle";
    if (strlen(lastCmdResult) > 0) doc["last_cmd"] = lastCmdResult;
    JsonObject metrics = doc["metrics"].to<JsonObject>();
    
    float airTemp = dht.readTemperature();
    float airHum = dht.readHumidity();
    
    bool dhtOk = !isnan(airTemp) && !isnan(airHum)
              && airTemp > -40.0f && airTemp < 80.0f
              && !(airTemp == 0.0f && airHum == 0.0f);  // E1: dual-zero = hardware fault
    
    // Retry once after 50ms if first read fails (WiFi interrupt can disrupt DHT22 timing)
    if (!dhtOk) {
        delay(50);
        airTemp = dht.readTemperature();
        airHum = dht.readHumidity();
        dhtOk = !isnan(airTemp) && !isnan(airHum)
              && airTemp > -40.0f && airTemp < 80.0f
              && !(airTemp == 0.0f && airHum == 0.0f);
        if (dhtOk) Serial.print("DHT22 retry OK | ");
    }
    
    if (dhtOk) {
        dhtFailCount = 0;
        metrics["air_temp"] = roundf(airTemp * 100.0f) / 100.0f;
        metrics["air_humidity"] = roundf(airHum * 100.0f) / 100.0f;
        metrics["dht_status"] = "ok";
        Serial.printf("气温: %.1f℃ | 气湿: %.1f%% | ", airTemp, airHum);
    } else {
        dhtFailCount++;
        Serial.printf("DHT22 异常(连续%d次) | ", dhtFailCount);
        if (dhtFailCount >= DHT_FAIL_THRESHOLD) {
            metrics["dht_status"] = "failed";
        }
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
    
    char json[448];
    size_t n = serializeJson(doc, json, sizeof(json));
    if (n >= sizeof(json)) {
        Serial.println("JSON 溢出!");
        return;
    }
    
    if (activeTransport == TRANSPORT_NONE) {
        // No active transport — buffer for later replay
        // (connection attempt from loop() will flush)
        mqttSeq++;
        appendToBuffer(json);
        Serial.println(" | 离线缓存");
        return;
    }
    
    publishMqttTelemetry(json);
    Serial.println();
}

// ==================== 离线缓冲区 ====================

void appendToBuffer(const char* line) {
    // 写入前检查剩余空间，不足时截断缓冲区
    if (LittleFS.totalBytes() - LittleFS.usedBytes() < 2048) {
        Serial.println("缓冲区空间不足，截断中...");
        File rf = LittleFS.open(BUFFER_FILE, "r");
        if (!rf) { LittleFS.remove(BUFFER_FILE); }
        else {
            // 统计行数
            int totalLines = 0, c;
            while ((c = rf.read()) >= 0) { if (c == '\n') totalLines++; }
            rf.close();
            if (totalLines > BUFFER_MAX_LINES / 2) {
                int skip = totalLines - BUFFER_MAX_LINES / 2;
                rf = LittleFS.open(BUFFER_FILE, "r");
                File wf = LittleFS.open(BUFFER_TMP, "w");
                if (rf && wf) {
                    char buf[384]; int lineNo = 0;
                    while (rf.available()) {
                        size_t len = rf.readBytesUntil('\n', buf, sizeof(buf)-1);
                        if (len == 0) continue; buf[len] = '\0'; lineNo++;
                        if (lineNo > skip) wf.println(buf);
                    }
                    rf.close(); wf.close();
                    LittleFS.remove(BUFFER_FILE);
                    LittleFS.rename(BUFFER_TMP, BUFFER_FILE);
                } else {
                    if (rf) rf.close(); if (wf) wf.close();
                    LittleFS.remove(BUFFER_FILE);
                }
            }
        }
    }
    File f = LittleFS.open(BUFFER_FILE, "a");
    if (!f) { Serial.println("缓冲区写入失败"); return; }
    f.println(line);
    f.close();
}

void flashFlushBuffer() {
    if (!LittleFS.exists(BUFFER_FILE)) return;
    if (activeTransport == TRANSPORT_NONE) return;
    
    File rf = LittleFS.open(BUFFER_FILE, "r");
    if (!rf) return;
    
    // 空文件直接删除
    if (rf.available() == 0) { rf.close(); LittleFS.remove(BUFFER_FILE); return; }
    
    // 尝试创建临时文件（空间不足时降级为丢弃模式）
    File wf = LittleFS.open(BUFFER_TMP, "w");
    bool canSave = wf;
    
    char line[384];
    int sent = 0, remaining = 0;
    
    while (rf.available()) {
        size_t len = rf.readBytesUntil('\n', line, sizeof(line)-1);
        if (len == 0) continue;
        line[len] = '\0';
        if (len > 0 && line[len-1] == '\r') line[--len] = '\0';
        
        {
            char topic[64];
            telemetryTopic(topic, sizeof(topic));
            bool ok = false;
            if (activeTransport == TRANSPORT_LAN_TCP) {
                ok = mqtt.publish(topic, line);
                mqtt.loop();  // 泵出 TCP 缓冲区，给下一条腾空间
            } else if (activeTransport == TRANSPORT_WAN_WS) {
                ok = wsSendMqttPublish(topic, (uint8_t*)line, strlen(line), (uint16_t)(++mqttSeq));
                webSocket.loop();
            }
            if (ok) { sent++; if (sent % 50 == 0) delay(1); continue; }
            delay(1);  // 发布失败时也让出 CPU
        }
        if (canSave) { wf.println(line); remaining++; }
    }
    
    rf.close();
    if (canSave) wf.close();
    LittleFS.remove(BUFFER_FILE);
    
    if (canSave && remaining > 0) {
        LittleFS.rename(BUFFER_TMP, BUFFER_FILE);
    } else if (canSave) {
        LittleFS.remove(BUFFER_TMP);
    }
    // cannotSave 模式：直接丢弃剩余数据，避免原地死锁
    
    if (sent > 0) Serial.printf("缓冲区: %d条已发送, %d条%s\n", sent, remaining,
        canSave ? "保留" : "丢弃(空间不足)");
}

// ==================== WiFi ====================

void resolveLanHost() {
    IPAddress ip = MDNS.queryHost("agri-server");
    if (ip) {
        snprintf(LAN_HOST, sizeof(LAN_HOST), "%d.%d.%d.%d", ip[0], ip[1], ip[2], ip[3]);
        Serial.printf("mDNS: agri-server.local → %s\n", LAN_HOST);
    } else {
        strncpy(LAN_HOST, "172.20.10.2", sizeof(LAN_HOST));
        LAN_HOST[sizeof(LAN_HOST)-1] = '\0';
        Serial.printf("mDNS: agri-server.local 未找到, 使用 %s\n", LAN_HOST);
    }
    lastLanResolve = millis();
}

void setupWiFi() {
    Serial.printf("连接 WiFi: %s\n", WIFI_SSID);
    WiFi.mode(WIFI_STA);
    // 打印 MAC 地址
    Serial.printf("主节点 MAC: %s\n", WiFi.macAddress().c_str());
    WiFi.begin(WIFI_SSID, WIFI_PASSWORD);
    WiFi.setAutoReconnect(true);
    
    for (int retry = 0; retry < 3; retry++) {
        int timeout = 30;
        while (WiFi.status() != WL_CONNECTED && timeout > 0) {
            delay(500); Serial.print("."); timeout--;
        }
        if (WiFi.status() == WL_CONNECTED) {
            Serial.printf("\nWiFi 已连接! IP: %s\n", WiFi.localIP().toString().c_str());
            if (MDNS.begin(NODE_ID)) Serial.println("mDNS 就绪");
            resolveLanHost();
            return;
        }
        Serial.printf("\nWiFi 重试 %d/3...\n", retry+1);
    }
    Serial.println("\nWiFi 失败，重启");
    ESP.restart();
}

// ==================== OTA 升级（MQTT 流式）====================
// 固件通过现有 WebSocket MQTT 连接以 MQTT PUBLISH 消息分块接收，
// 在 handleMqttPacket() 中直接写入 Flash。无需 HTTP/TLS 连接，
// 绕过了 Funnel 同一 IP 并发 TLS 连接限制。

// ==================== 初始化与主循环 ====================

void setup() {
    Serial.begin(115200);
    Serial.printf("\n=== 农业物联网 ESP32 固件 v%s (%s) ===\n", FW_VERSION, NODE_ID);
    
    pinMode(RELAY_PIN, OUTPUT);
    digitalWrite(RELAY_PIN, LOW);
    
    pinMode(RS485_DIR, OUTPUT);
    digitalWrite(RS485_DIR, LOW);
    
    soilSerial.begin(soilBaud, SERIAL_8N1, RS485_RX, RS485_TX);
    dht.begin();
    wanTls.setInsecure();
    
    mqtt.setBufferSize(MQTT_BUF_SIZE);
    Serial.printf("MQTT 缓冲区: %d\n", mqtt.getBufferSize());
    
    scanSoilSensor();
    
    // LittleFS 挂载。挂载失败时格式化（旧数据可能损坏），挂载成功则保留缓冲数据跨重启恢复。
    if (!LittleFS.begin()) {
        Serial.println("LittleFS 挂载失败，格式化...");
        LittleFS.format();
        if (!LittleFS.begin()) {
            Serial.println("LittleFS 格式化后仍挂载失败");
        } else {
            Serial.println("LittleFS 格式化并挂载成功");
        }
    } else {
        Serial.println("LittleFS 挂载成功，保留缓冲数据");
    }
    
    // 生成本次启动的唯一标识
    uint32_t r = esp_random();
    snprintf(bootId, sizeof(bootId), "%08lx", (unsigned long)r);
    Serial.printf("Boot ID: %s\n", bootId);

    // 保留旧缓冲区 (/buffer.dat) 跨重启恢复数据。
    // 连接建立后 flashFlushBuffer() 会自动回放。
    // (格式化后缓冲区为空，无数据回放)

    setupWiFi();

    // NTP 时间同步（用于 captured_at 采集时间戳）
    configTime(8 * 3600, 0, "pool.ntp.org", "time.nist.gov");
    {
        time_t now = 0;
        int retries = 0;
        while (now < 100000 && retries < 20) {
            delay(500);
            time(&now);
            retries++;
        }
        if (now >= 100000) {
            Serial.printf("NTP 同步成功: %s", ctime(&now));
        } else {
            Serial.println("NTP 同步超时，将使用服务端时间");
        }
    }
}

// ==================== 主循环 ====================

void loop() {
    unsigned long now = millis();
    
    // 检测 WiFi 重新连接 → 强制 MQTT 重连（防止 TCP 客户端撒谎说还连着）
    wl_status_t curWifi = WiFi.status();
    if (curWifi == WL_CONNECTED && lastWifiStatus != WL_CONNECTED) {
        Serial.println("WiFi 重连，强制断开旧 MQTT");
        activeTransport = TRANSPORT_NONE;
        lanTcp.stop();
        wanWsConnected = false;
        wanMqttConnected = false;
        lastMqttReconnect = 0;
        LAN_HOST[0] = '\0';  // 服务器 IP 可能已变，下次连接重新 mDNS 解析
    }
    // 记录 WiFi 在线时间戳（用于断线时降频省电）
    if (curWifi == WL_CONNECTED) lastWifiOnline = millis();
    lastWifiStatus = curWifi;
    
    // 主动重连 WiFi（断线时按指数退避尝试）
    if (curWifi != WL_CONNECTED) {
        ensureWiFi();
    } else {
        // 保持 MQTT 连接
        if (activeTransport == TRANSPORT_LAN_TCP) {
            mqtt.loop();
        } else if (activeTransport == TRANSPORT_WAN_WS) {
            webSocket.loop();
        }
        if (now - lastMqttReconnect >= MQTT_RECONNECT_INTERVAL) {
            ensureMqttConnected();
        }
    }

    // OTA 由 WebSocket MQTT 事件中的 handleMqttPacket() 直接处理，
    // 固件块在回调中写入 Flash。无需在主循环中做额外处理。
    
    // 遥测静默看门狗：TELEMETRY_SILENT_TIMEOUT 无成功发布则强制重连
    if (curWifi == WL_CONNECTED && lastTelemetryOk > 0
        && now - lastTelemetryOk > TELEMETRY_SILENT_TIMEOUT
        && now - lastRead > 30000)  // 避免 DHT22 采集期间误触发
    {
        Serial.printf("看门狗: 遥测静默 %lus，强制重连\n", (now - lastTelemetryOk) / 1000);
        lastTelemetryOk = now;  // 给下次重连一个宽限期
        lastRead = now;         // 重置读周期，避免立即再触发
        activeTransport = TRANSPORT_NONE;
        lanTcp.stop();
        wanWsConnected = false;
        wanMqttConnected = false;
    }

    // 传感器采集间隔：WiFi 在线时 10s，断线时逐步降频省电
    unsigned long readInterval = READ_INTERVAL;
    if (curWifi != WL_CONNECTED && lastWifiOnline > 0) {
        unsigned long offlineDuration = (millis() - lastWifiOnline) / 1000;
        if (offlineDuration > 300)          readInterval = 300000;   // >5min → 5分钟
        else if (offlineDuration > 60)      readInterval = 60000;    // >1min → 1分钟
        // 否则保持 10 秒（前 1 分钟快速采样，积极等待重连）
    }
    if (now - lastRead >= readInterval) {
        lastRead = now;
        publishTelemetry();
    }
}
