# Agri-iot 项目笔记

## 项目概述
农业物联网监控系统，Rust 技术栈：
- **agri-core**: 核心库（模型、DB、错误定义、共享遥测处理）
- **agri-server**: 后端服务（Axum + SQLx + 响应辅助函数）
- **agri-mqtt**: MQTT 通信（rumqttd broker + rumqttc client）
- **agri-ui**: React SPA（TypeScript + Ant Design + ECharts）
- **esp32-firmware**: ESP32 固件 v2.1（RS485 土壤三合一）

## AI 决策系统重构（2026-05-18）

### 核心理念
```
传感器数据 + 气象数据 + 作物知识库
         ↓
    AI 决策中枢
         ↓
    环境调控 + 紧急保护
         ↓
    知识积累（越用越专业）
```

### 知识库引擎：Obsidian
- **Vault 路径**: `OBSIDIAN_VAULT_PATH` 环境变量
- **结构**: 00-Crops/ 01-Pests/ 02-Cases/ 03-Weather/ 04-Templates/ 05-Daily/
- **特点**: 双向链接、Markdown、CLI 操作、模板系统
- **数据流**: Rust后端 → 文件系统 → Obsidian Vault → AI检索

### 紧急保护（优先级最高）
| 情况 | 自动动作 | 确认 | 通知级别 |
|------|----------|------|----------|
| 大风(>40km/h) | 关闭顶部通风 | ❌ | 高 |
| 大雨(>10mm/h) | 关闭顶部通风 | ❌ | 高 |
| 下雪 | 关闭通风+暂停自动 | ❌ | CRITICAL |

### 决策流程
1. 数据融合（Sensor + Weather）
2. 紧急检测（最高优先级）
3. 环境评估（评分系统）
4. 知识检索（Obsidian RAG）
5. 决策输出（建议/执行）

### 设备控制
- 卷膜器（顶部/侧面通风）：0-100% 量程
- 量程需落地时学习
- EC值监测（人工干预为主）

### 文档
- 架构设计: `ARCHITECTURE-AI-DECISION.md`
- 包含完整知识库模板、紧急规则、AI决策逻辑

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

## ESP32 固件 v2.1（2026-05-19）

### 变更（v2.0 → v2.1）
- 移除旧版 ADC 土壤湿度 + 光照传感器
- 新增 RS485 (Modbus RTU) 土壤传感器，通过 UART2 获取土壤温度/湿度/EC
- 使用 MAX485 模块，DE/RE 合接 GPIO4 做方向控制
- 继电器从 GPIO16 移至 GPIO2（避免与 UART2 RX 冲突）

### 数据通路
```
ESP32 (DHT22 + RS485 土壤三合一)
  → WiFi ("iPhone")
  → HTTPS → zero-1.taile2b316.ts.net/api/v1/telemetry
  → Tailscale Funnel → http://172.20.10.2:3001 → agri-server → DB
```

- **引脚定义**
  | 引脚 | 设备 |
  |------|------|
  | GPIO15 | DHT22 空气温湿度 |
  | GPIO16 | RS485 RX (MAX485 RO) |
  | GPIO17 | RS485 TX (MAX485 DI) |
  | GPIO4  | RS485 DIR (DE+RE) |
  | GPIO2  | 继电器 |

- **关键特性**
  - HTTP 直连（非 MQTT），走 Tailscale Funnel
  - 每 10 秒采集传感器，每 3 秒轮询命令
  - Modbus RTU 协议（地址 0x01，波特率 4800，CRC16 校验）
  - 上报字段：`air_temp`、`air_humidity`、`soil_temp`、`soil_moisture`、`ec`、`light`、`temperature`、`humidity`
- 指令：`switch`（继电器开关）、命令完成 PUT 回执

### 已知 Bug（已修复）
- 服务端 `get_pending_commands` 返回 `id` 为整数，ESP32 用 `const char*` 接收时 null → `LoadProhibited` panic
- 修复：`routes.rs:274` → `"id": r.0.to_string()`

## 前端（2026-05-20）

### React SPA（生产）
- `agri-ui/` 为 Vite + React + TypeScript + Ant Design + ECharts 项目
- 预构建产物部署在 `agri-server/static/`
- 通过 SSE `/api/v1/events` 接收实时数据推送

### 完成页面
- Dashboard (ECharts)
- 区域列表/详情 (ZoneList, ZoneDetail)
- 节点列表 (NodeList)
- 规则管理 (RuleList)
- 数据查询 (DataQuery)
- 系统设置 (Settings)
- 天气面板 (WeatherPanel)
- 设备控制面板 (ControlPanel)

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
| POST | `/api/v1/ai/assess` | AI 环境评估 |
| GET | `/api/v1/ai/emergency/status` | 紧急情况状态 |
| GET | `/api/v1/ai/knowledge/search` | 知识库搜索 |
| GET/POST | `/api/v1/ai/knowledge/cases` | 调控案例管理 |
| GET | `/api/v1/ai/knowledge/obsidian/note` | Obsidian 笔记读取 |
| GET | `/api/v1/ai/knowledge/obsidian/search` | Obsidian 搜索 |
| POST | `/api/v1/ai/knowledge/obsidian/case` | 添加案例到 Obsidian |
| GET | `/api/v1/ai/ventilation/config/:area_id` | 通风配置查询 |
| POST | `/api/v1/ai/ventilation/calibrate/:device_id` | 卷膜器校准 |
| GET | `/api/v1/ai/ec/analyze/:area_id` | EC 分析 |
| POST | `/api/v1/ai/control/ventilation` | 手动控制通风 |
| GET | `/api/v1/weather/now` | 实时天气 |
| GET | `/api/v1/weather/3d` | 3 天预报 |
| GET | `/api/v1/weather/24h` | 24 小时预报 |
| GET | `/api/v1/weather/minutely` | 分钟级降水 |
| GET | `/api/v1/weather/air` | 空气质量 |
| GET | `/api/v1/weather/indices` | 生活指数 |
| GET | `/api/v1/weather/warning` | 灾害预警 |
| GET | `/api/v1/weather/geo` | 城市查找 |

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
- LLD 链接器与工具链版本可能不兼容，`cargo clean` 后用 `CARGO_BUILD_JOBS=1` 可缓解内存不足
- 内存仅 1.8GB，并行 rustc 进程容易 OOM，编译需单线程或低并行度
- `ObsidianKnowledge::safe_path()` 需要 vault 路径存在才能 canonicalize，否则回退到字符串过滤

## 审查修复记录（2026-05-19）

### 已修复
- **Migration 002_zones.sql 未被执行**：根 `migrations/` 下有 `002_zones.sql`，但 `agri-core/migrations/` 未链接 → 新建 symlink 修复
- **Migration 目录脏文件**：删除了 artifact `001_init.sql~bab0449...`
- **测试 DB 缺少基础表**：`ai_routes` 测试缺 `devices`/`sensor_readings`，`rule_engine` 测试缺 `weather_data` → 补齐后 123/123 测试全过

### 测试统计
- `agri-core`: 92 测试（models 32 + ai 52 + error 8）
- `agri-server`: 32 测试（routes 11 + rule_engine 12 + ai_routes 9）
- `agri-mqtt`: 22 测试（handler 10 + mqtt 12）
- **总计: 146 测试**

## 全面审查修复（2026-05-20）

### 前端清理
- **`agri-frontend/` 删除**：JSX + Recharts 版本移除，保留 `agri-ui/`（TypeScript + Ant Design + ECharts）
- **`deploy/build.sh`**：原已指向 `agri-ui`（验证正确）
- **README/AGENTS.md**：更新前端引用、项目结构、测试计数

### 已修复

| # | 问题 | 修复 |
|---|------|------|
| 1 | **AI 评估静默失效** — `ingest_telemetry` 归一化 `air_temp→temperature` 等，但 `assess` 匹配旧名 | `ai_routes.rs:112-121` 改为匹配归一化后名称 `temperature`/`humidity`/`soil_temperature` |
| 2 | **Obsidian 路径穿越** — `vault_path.join(path)` 中 `path=/etc/passwd` 可读取任意文件 | `knowledge.rs` 新增 `safe_path()`：拒绝 `..` 和绝对路径，canonicalize 后验证在 vault 内 |
| 3 | **MQTT/HTTP 遥测不一致** — 同传感器数据因入口不同产生不同字段名和单位 | 新建 `agri-core/src/telemetry.rs`，共享 `process_telemetry()`/`normalize_metric()`/`validate_value()` |
| 4 | **SystemFailure 告警风暴** — 设备离线 >30min 后每 5 秒写入重复告警 | `emergency.rs` 添加 `system_failure_fired_at` Map，60 分钟冷却期内不重复触发 |
| 5 | **迁移目录分裂** — 根 `migrations/` 与 `agri-core/migrations/` 重复，`002_zones.sql` 创建废弃表 | 删除根 `migrations/`；删除 `002_zones.sql`；`003_ai_knowledge.sql` → `002_ai_knowledge.sql`；移除手动 `ensure_ai_tables()` |
| 6 | **错误响应重复** — 约 30 处完全相同的 `(StatusCode, Json(...)).into_response()` | 新建 `response.rs`：`ok_json()`/`err_json()`/`internal_err()`/`not_found()`/`bad_request()` |

### 变更文件清单
```
新增: agri-core/src/telemetry.rs          # 共享遥测处理
新增: agri-server/src/response.rs         # 响应辅助函数
修改: agri-core/Cargo.toml                # 添加 tracing 依赖
修改: agri-core/src/lib.rs                # 注册 telemetry 模块
修改: agri-core/src/db.rs                 # 移除 ensure_ai_tables
修改: agri-core/src/ai/knowledge.rs       # safe_path 路径穿越防护
修改: agri-core/src/ai/emergency.rs       # SystemFailure 去重
修改: agri-mqtt/src/handler.rs            # 使用共享 telemetry
修改: agri-server/src/main.rs             # 注册 response 模块
修改: agri-server/src/routes.rs           # 使用共享 telemetry + 响应辅助函数
修改: agri-server/src/ai_routes.rs        # 修复评估字段名 + 响应辅助 + Obsidian 安全
迁移: agri-core/migrations/               # 001_init.sql 重建, 003→002, 删除 002_zones.sql
删除: agri-frontend/                      # 整个目录
删除: migrations/                         # 根目录重复迁移
文档: README.md, AGENTS.md                # 更新结构和测试统计
```
