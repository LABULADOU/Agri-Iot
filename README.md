# Agri-Iot — 农业物联网监控系统

> 项目已从 **Agri-lot** 重命名为 **Agri-Iot**（2026-06-08）。
> 远程仓库：`https://github.com/LABULADOU/Agri-Iot.git`

基于 Rust 的农业物联网监控系统，支持传感器数据采集、规则引擎、实时监控和 Tailscale Funnel 远程接入。

## 架构

```
┌──────────────────────────────────────────────────────────┐
│  ESP32 v3.0 (MQTT 双通道)                                │
│  DHT22 + RS485 土壤三合一（温度/湿度/EC）+ 继电器         │
│  ├── LAN:  MQTT TCP → agri-server.local:1883            │
│  └── WAN:  WebSocket MQTT → wss://zero-1.../mqtt        │
│                     ↓                                    │
│              agri-server:3001/mqtt                        │
│              (WebSocket ↔ MQTT TCP 代理)                  │
│                     ↓                                    │
│              rumqttd broker (127.0.0.1:1883)              │
│                     ↓                                    │
│              agri-mqtt handler (QoS 1)                    │
│                     ↓                                    │
│              process_telemetry() → SQLite + SSE           │
└──────────────────────────────────────────────────────────┘

串口模式: ESP32 USB → serial_bridge.py → POST /api/v1/telemetry
```

| 组件                 | 技术栈                     | 说明                   |
| ------------------ | ----------------------- | -------------------- |
| **agri-core**      | Rust                    | 核心类型、数据库工具、错误定义      |
| **agri-server**    | Rust + Axum + SQLx      | HTTP API 服务、规则引擎、WebSocket 桥接 |
| **agri-mqtt**      | Rust + rumqttd/rumqttc  | MQTT Broker（独立进程）和客户端     |
| **agri-ui**        | React + TypeScript + Ant Design + ECharts | 前端 SPA（预构建到 static/） |
| **esp32-firmware** | Arduino + ESP32         | 传感器采集 + 纯 MQTT（v3.0）      |
| **serial_bridge**  | Python                  | ESP32 串口数据 → HTTP 桥接 |

## 快速启动

### 1. 环境要求

- Rust 1.75+
- Python 3（串口桥接）
- SQLite（自动创建）
- Tailscale（可选，用于远程 Funnel 接入）

### 2. 配置

```bash
cp .env.example .env
# 编辑 .env，配置 WEATHER_API_KEY（和风天气）
```

### 3. 构建并启动

```bash
# 构建
cargo build -p agri-server -p agri-mqtt --bin broker

# 启动服务（进程管理器同时托管 broker + server）
./scripts/init.sh

# 或后台运行
nohup ./scripts/init.sh &

# 或指定构建类型
BUILD_TYPE=release nohup ./scripts/init.sh &
```

访问 http://localhost:3001

### 4. 手动启动（不通过进程管理器）

```bash
# 终端 1: 启动独立 broker
./target/debug/broker

# 终端 2: 启动 server
MQTT_BROKER_ADDR=127.0.0.1:1883 ./target/debug/agri-server
```

### 5. 数据接入

**串口桥接（真实 ESP32）**：

```bash
sudo python3 scripts/serial_bridge.py /dev/ttyUSB0
```

## Tailscale Funnel 远程接入

ESP32 固件通过 Tailscale Funnel 连接公网地址，无需内网穿透配置：

```
https://zero-1.taile2b316.ts.net
         ↓  Funnel (WSL)
http://172.20.10.13:3001 (container)
         ↓
    agri-server
```

## API 概览

| 方法             | 路径                                | 说明              |
| GET            | `/api/v1/dashboard/summary`       | 仪表盘汇总           |
| GET            | `/api/v1/dashboard/area-readings` | 分区图表数据          |
| GET            | `/api/v1/dashboard/node-readings` | 节点实时数据          |
| GET/POST       | `/api/v1/devices`                 | 设备列表/创建（UPSERT） |
| GET/PUT/DELETE | `/api/v1/devices/:id`             | 设备详情/更新/删除      |
| GET            | `/api/v1/devices/:id/readings`    | 传感器历史数据         |
| POST           | `/api/v1/devices/:id/command`     | 发送控制指令          |
| POST           | `/api/v1/telemetry`               | 遥测数据接入          |
| GET            | `/api/v1/commands/node/:node_id`  | 查询待处理命令         |
| PUT            | `/api/v1/commands/:id/status`     | 更新命令状态          |
| GET/POST       | `/api/v1/rules`                   | 规则列表/创建         |
| PUT/DELETE     | `/api/v1/rules/:id`               | 规则更新/删除         |
| GET            | `/api/v1/alerts`                  | 告警/命令日志         |
| GET            | `/api/v1/system/info`             | 系统信息            |
| GET            | `/api/v1/events`                  | SSE 实时事件推送      |
| CRUD           | `/api/v1/areas`                   | 区域管理            |
| CRUD           | `/api/v1/crops`                   | 作物管理            |
| CRUD           | `/api/v1/crop-batches`            | 茬口管理            |
| POST           | `/api/v1/ai/assess`               | AI 环境评估          |
| GET            | `/api/v1/ai/emergency/status`     | 紧急情况状态          |
| GET            | `/api/v1/ai/knowledge/search`     | 知识库搜索           |
| GET/POST       | `/api/v1/ai/knowledge/cases`      | 调控案例管理          |
| POST           | `/api/v1/ai/ventilation/calibrate/:device_id` | 卷膜器校准 |
| GET            | `/api/v1/ai/ventilation/config/:area_id`      | 通风配置查询 |
| GET            | `/api/v1/ai/ec/analyze/:area_id`  | EC 分析            |
| POST           | `/api/v1/ai/control/ventilation`  | 手动控制通风          |
| GET            | `/api/v1/weather/now`             | 实时天气            |
| GET            | `/api/v1/weather/3d`              | 3 天预报           |
| GET            | `/api/v1/weather/24h`             | 24 小时预报         |
| GET            | `/api/v1/weather/minutely`        | 分钟级降水           |
| GET            | `/api/v1/weather/air`             | 空气质量            |
| GET            | `/api/v1/weather/indices`         | 生活指数            |
| GET            | `/api/v1/weather/warning`         | 灾害预警            |
| GET            | `/api/v1/weather/geo`             | 城市查找            |

> SSE 事件由 `POST /api/v1/telemetry` 触发，通过 `broadcast::Sender` 推送到所有 SSE 客户端。
> 天气接口为和风天气 API 的反向代理，`safe_proxy()` 对免费套餐不支持的端点返回空数据而非 502。

## 项目结构

```
agri-core/src/          # 核心库
├── models.rs           # 数据模型（capabilities 字段）
├── telemetry.rs        # 遥测处理（归一化/验证/写入 — 共享 MQTT+HTTP）
├── db.rs               # 数据库连接和迁移
├── error.rs            # 错误类型定义
└── ai/                 # AI 决策系统
    ├── assess.rs       # 环境评估（评分系统）
    ├── emergency.rs    # 紧急情况检测（大风/雨/雪）
    ├── ventilation.rs  # 通风决策
    ├── fertigation.rs  # EC 分析
    ├── night_mode.rs   # 夜间模式
    ├── calibration.rs  # 卷膜器校准
    └── knowledge.rs    # Obsidian 知识库 RAG

agri-server/src/        # 后端服务
├── main.rs             # 入口
├── routes.rs           # API 路由
├── response.rs         # 响应辅助函数（ok_json/err_json/internal_err）
├── areas.rs            # 区域/作物/茬口管理
├── state.rs            # AppState（含 telemetry_limiter）
├── rule_engine.rs      # 规则引擎
├── weather.rs          # 天气 API 代理 (和风天气)
├── ai_routes.rs        # AI 决策 API 路由
├── mqtt_ws.rs          # WebSocket ↔ MQTT TCP 桥接
├── rate_limiter.rs     # 遥测速率限制
└── request_logger.rs   # 请求日志

agri-mqtt/src/          # MQTT 通信
├── bin/
│   └── broker.rs       # 独立 rumqttd broker 二进制
├── broker.rs           # 嵌入式 MQTT Broker（备用）
├── client.rs           # MQTT 客户端
└── handler.rs          # 遥测/状态处理（QoS 1 + 通道解耦 + seq 去重）

agri-ui/                # React SPA (TypeScript + Ant Design + ECharts)
├── src/pages/          # 页面组件
├── src/components/     # 通用组件
├── src/services/       # API 服务封装
└── build → agri-server/static/

esp32-firmware/src/     # ESP32 固件
└── main.ino            # v3.0: 纯 MQTT（PubSubClient + WebSocket MQTT）

scripts/                # 工具脚本
├── init.sh             # 进程管理器（托管 broker + server）
├── serial_bridge.py    # 串口桥接
├── mdns_advertise.py   # Python raw mDNS responder
├── start_mdns.sh       # mDNS 启动脚本
├── stress_test.py      # MQTT 压力测试
└── run_bridge.sh       # 串口桥接启动脚本

agri-core/migrations/   # 数据库迁移（单一来源）
├── 001_init.sql        # 基础表（devices, sensor_readings 等）
├── 002_ai_knowledge.sql # AI 知识库 + 气象 + 评估表
└── 003_dedup.sql       # seq 列 + 部分唯一索引（MQTT 去重）
```

## 设备模型

一块物理 ESP32 对应一条设备记录，通过 `capabilities` JSON 字段描述功能：

```json
{
  "id": "uuid",
  "node_id": "esp32-node-001",
  "device_type": "sensor",
  "capabilities": ["sensor", "actuator"],
  "status": "online"
}
```

> **自动注册**：MQTT handler 在收到第一条遥测时自动创建设备记录（`devices` 表），无需手动注册。
> ESP32 WAN WebSocket 超时已从 5s 调整为 30s，补偿 TLS handshake 延迟。
> 仪表盘同时显示已分配和未分配区域的设备。
> **断线容错**：handler 使用 `clean_session=false`，agri-server 重启后 broker 自动回放离线期间的 QoS 1 消息（含 LittleFS 本地缓存双重保障）。

## 开发

```bash
# 编译检查
cargo check -p agri-server -p agri-mqtt -p agri-core

# 运行全部测试
cargo test -p agri-core   # 92 测试
cargo test -p agri-server # 32 测试
cargo test -p agri-mqtt   # 22 测试

# 内存泄漏检测
cargo test --release -p agri-server 2>&1  # 编译慢但结果更可靠

# 前端
cd agri-ui && npm run build  # 生产构建 → agri-server/static/
cd agri-ui && npm run dev    # 开发模式（Vite HMR）
```
