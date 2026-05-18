# Agri-iot — 农业物联网监控系统

基于 Rust 的农业物联网监控系统，支持传感器数据采集、规则引擎、实时监控和 Tailscale Funnel 远程接入。

## 架构

```
┌─────────────────────────────────────────────────────┐
│  ESP32 真实节点 (HTTP+Funnel)                        │
│  DHT22 + 土壤湿度 + 光照 + 继电器                    │
│  └── HTTPS → zero-1.taile2b316.ts.net/api/v1/*       │
│         ↕ Tailscale Funnel                           │
│         → http://172.20.10.2:3001 → agri-server      │
└─────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────┐
│  MQTT 模拟器 (本地测试)                        │
│  └── MQTT → rumqttd Broker → agri-server      │
└──────────────────────────────────────────────┘

┌──────────────────────────────────────────────┐
│  ESP32 (串口/USB)                             │
│  └── USB → serial_bridge.py → HTTP → server   │
└──────────────────────────────────────────────┘
```

| 组件 | 技术栈 | 说明 |
|------|--------|------|
| **agri-core** | Rust | 核心类型、数据库工具、错误定义 |
| **agri-server** | Rust + Axum + SQLx | HTTP API 服务、规则引擎 |
| **agri-mqtt** | Rust + rumqttd/rumqttc | MQTT Broker 和客户端 |
| **agri-frontend** | React + Vite + Recharts | 前端 SPA（预构建到 static/） |
| **esp32-firmware** | Arduino + ESP32 | 传感器采集 + HTTP 直连 |
| **serial_bridge** | Python | ESP32 串口数据 → HTTP 桥接 |

## 快速启动

### 1. 环境要求

- Rust 1.75+
- Python 3（模拟器/串口桥接）
- SQLite（自动创建）
- Tailscale（可选，用于远程 Funnel 接入）

### 2. 配置

```bash
cp .env.example .env
```

### 3. 构建并启动

```bash
# 构建
cargo build -p agri-server

# 启动服务
./target/debug/agri-server

# 或后台运行
nohup ./target/debug/agri-server > /tmp/agri-server.log 2>&1 &
```

访问 http://localhost:3001

### 4. 数据接入

**HTTP 模拟器（推荐）**：
```bash
python3 scripts/simulate_http.py
```

**MQTT 模拟器**：
```bash
python3 scripts/simulate_node.py
```

**串口桥接（真实 ESP32）**：
```bash
python3 scripts/serial_bridge.py /dev/ttyUSB0
```

## Tailscale Funnel 远程接入

ESP32 固件通过 Tailscale Funnel 连接公网地址，无需内网穿透配置：

```
https://zero-1.taile2b316.ts.net
         ↓  Funnel
http://172.20.10.2:3001 (内网)
         ↓
    agri-server
```

## API 概览

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/v1/dashboard/summary` | 仪表盘汇总 |
| GET | `/api/v1/dashboard/area-readings` | 分区图表数据 |
| GET/POST | `/api/v1/devices` | 设备列表/创建（UPSERT） |
| GET/PUT/DELETE | `/api/v1/devices/:id` | 设备详情/更新/删除 |
| GET | `/api/v1/devices/:id/readings` | 传感器历史数据 |
| POST | `/api/v1/devices/:id/command` | 发送控制指令 |
| POST | `/api/v1/telemetry` | 遥测数据接入 |
| GET | `/api/v1/commands/node/:node_id` | 查询待处理命令 |
| PUT | `/api/v1/commands/:id/status` | 更新命令状态 |
| GET/POST | `/api/v1/rules` | 规则列表/创建 |
| PUT/DELETE | `/api/v1/rules/:id` | 规则更新/删除 |
| GET | `/api/v1/alerts` | 告警/命令日志 |
| GET | `/api/v1/system/info` | 系统信息 |
| GET | `/api/v1/events` | SSE 实时事件推送 |
| CRUD | `/api/v1/areas` | 区域管理 |
| CRUD | `/api/v1/crops` | 作物管理 |
| CRUD | `/api/v1/crop-batches` | 茬口管理 |

## 项目结构

```
agri-core/src/          # 核心库
├── models.rs           # 数据模型（capabilities 字段）
├── db.rs               # 数据库连接和迁移
└── error.rs            # 错误类型定义

agri-server/src/        # 后端服务
├── main.rs             # 入口
├── routes.rs           # API 路由
├── areas.rs            # 区域/作物/茬口管理
├── state.rs            # AppState
├── rule_engine.rs      # 规则引擎
└── request_logger.rs   # 请求日志

agri-mqtt/src/          # MQTT 通信
├── broker.rs           # 嵌入式 MQTT Broker
├── client.rs           # MQTT 客户端
└── handler.rs          # 遥测/状态处理

agri-frontend/          # React SPA (Vite)
├── src/pages/          # 页面组件
├── src/components/     # 通用组件
└── prebuilt → agri-server/static/

esp32-firmware/src/     # ESP32 固件
└── main.ino            # HTTP+Funnel 模式

scripts/                # 工具脚本
├── simulate_http.py    # HTTP 模拟器
├── simulate_node.py    # MQTT 模拟器
└── serial_bridge.py    # 串口桥接
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

## 开发

```bash
# 编译检查
cargo check -p agri-server -p agri-mqtt -p agri-core

# 运行测试
cargo test -p agri-core
cargo test -p agri-mqtt
cargo test -p agri-server

# 前端
cd agri-frontend && npm run dev
```
