# Agri-iot 项目笔记

## 项目概述
农业物联网监控系统，Rust 技术栈：
- **agri-core**: 核心库（模型、DB、错误定义）
- **agri-server**: 后端服务（Axum + SQLx）
- **agri-mqtt**: MQTT 通信（rumqttd broker + rumqttc client）
- **agri-frontend**: React SPA（Vite + Recharts）
- **esp32-firmware**: ESP32 固件 v2.0（HTTP + Tailscale Funnel）

## 架构

```
ESP32 真实节点 ───HTTPS──→ Tailscale Funnel ──→ agri-server (公网可达)
MQTT 模拟器     ───MQTT──→ rumqttd Broker   ──→ agri-server (本地)
ESP32(串口)     ───串口──→ serial_bridge.py ─HTTP→ agri-server (USB直连)
```

## 数据库重构 — 设备 capabilities 模型（2026-05-17）

### 背景
旧模型将一块物理 ESP32 拆为 sensor + actuator 两条记录，共用 node_id，导致 KPI 翻倍。

### 变更
- **`agri-core/migrations/001_init.sql`** → 原表结构，`node_id` 无 UNIQUE
- **`agri-core/src/db.rs`** → Rust 启动代码幂等补充：
  - `capabilities TEXT` 列（JSON 数组，如 `["sensor","actuator"]`）
  - `UNIQUE INDEX idx_devices_node_id`
  - 合并旧 DB 中 sensor+actuator 重复记录
- **`agri-core/src/models.rs`** → `Device` 增加 `capabilities: Option<JsonValue>`，`has_capability()` 方法
- **`agri-server/src/routes.rs`**：
  - `POST /api/v1/devices` → UPSERT 模式，同 node_id 自动更新
  - `send_command` → 检查 `capabilities` 包含 `"actuator"` 而非 device_type
  - `ingest_telemetry` → 不再按 `device_type = 'sensor'` 过滤
  - `get_pending_commands` → `id` 返回字符串（修复 ESP32 空指针崩溃）
- **`agri-mqtt/src/handler.rs`** → 同上去除 device_type 过滤

### 效果
- 一块板子 = 一个设备，KPI 正确
- 未来扩展（摄像头、屏幕等）只需追加 capabilities 数组

## ESP32 固件 v2.0（2026-05-18）

### 数据通路
```
ESP32 (DHT22 + 土壤湿度 + 光照 + 继电器)
  → WiFi ("iPhone")
  → HTTPS → zero-1.taile2b316.ts.net/api/v1/telemetry
  → Tailscale Funnel → http://172.20.10.2:3001 → agri-server → DB
```

### 关键特性
- `esp32-firmware/src/main.ino`：HTTP 直连（非 MQTT），走 Tailscale Funnel
- 每 10 秒采集传感器，每 3 秒轮询命令
- `setInsecure()` 跳过 SSL 验证（Tailscale 自有证书）
- 指令：`switch`（继电器开关）、命令完成 PUT 回执

### 已知 Bug（已修复）
- 服务端 `get_pending_commands` 返回 `id` 为整数，ESP32 用 `const char*` 接收时 null → `LoadProhibited` panic
- 修复：`routes.rs:274` → `"id": r.0.to_string()`

## 前端（2026-05-18）

### React SPA（生产）
- `agri-frontend/` 为 Vite + React + Recharts + Tailwind CSS 项目
- 预构建产物部署在 `agri-server/static/`
- 通过 SSE `/api/v1/events` 接收实时数据推送

### 完成页面
- 概览/仪表盘
- 设备列表/详情
- 告警记录
- 规则管理
- 系统设置
- 区域管理（ZoneDetail + FarmScene 3D 可视化）

## 后端 API 端点

| 方法 | 路径 | 说明 |
|------|------|------|
| GET/POST | `/api/v1/devices` | 设备列表/创建（UPSERT） |
| GET/PUT/DELETE | `/api/v1/devices/:id` | 设备详情/更新/删除 |
| GET | `/api/v1/devices/:id/readings` | 传感器历史数据 |
| POST | `/api/v1/devices/:id/command` | 发送控制指令（检查 capabilities） |
| POST | `/api/v1/telemetry` | 遥测数据接入（HTTP 直连） |
| GET | `/api/v1/commands/node/:node_id` | 查询待处理命令 |
| PUT | `/api/v1/commands/:id/status` | 更新命令状态 |
| CRUD | `/api/v1/areas` | 区域管理 |
| CRUD | `/api/v1/crops` | 作物管理 |
| CRUD | `/api/v1/crop-batches` | 茬口管理 |
| GET | `/api/v1/dashboard/summary` | 仪表盘汇总 |
| GET | `/api/v1/dashboard/area-readings` | 分区图表数据 |
| GET | `/api/v1/dashboard/node-readings` | 节点实时数据 |
| GET | `/api/v1/system/info` | 系统信息 |
| GET | `/api/v1/events` | SSE 实时事件推送 |

## 启动方式

```bash
# 构建
cargo build -p agri-server

# 启动服务
./target/debug/agri-server

# 或作为后台进程
nohup ./target/debug/agri-server > /tmp/agri-server.log 2>&1 &

# 模拟传感器（本地 MQTT）
python3 scripts/simulate_node.py

# 模拟传感器（HTTP 直连）
python3 scripts/simulate_http.py

# 真实 ESP32 串口桥接
python3 scripts/serial_bridge.py /dev/ttyUSB0
```

Dashboard: http://localhost:3001
Tailscale Funnel: https://zero-1.taile2b316.ts.net

## 已知坑点

- `rumqttc` 0.24 的 `AsyncClient` 和 `EventLoop` 要分开创建
- SQLite `enabled` 字段是 `INTEGER`，与 `bool` 比较用 `== 1i64`
- 命令轮询返回的 `id` 必须是字符串（ESP32 `const char*` 接收），否则崩溃
- 设备状态无超时离线检测（需 telemetry 或 MQTT 消息才更新）
- `mosquitto` 子进程启动后即退出（不影响 MQTT 功能，但需排查）
