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
  → Tailscale Funnel (WSL) → 172.20.10.13:3001 → agri-server → DB
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

## 前端（2026-05-22）

### React SPA（生产）
- `agri-ui/` 为 Vite + React + TypeScript + Ant Design + ECharts 项目
- 预构建产物部署在 `agri-server/static/`
- 通过 SSE `/api/v1/events` 接收实时数据推送
- 后端 metric 归一化：ESP32 发送 `temperature`/`humidity`，前端指标名需与 DB 一致

### 实时数据流
```
ESP32 → POST /api/v1/telemetry → process_telemetry() → DB写入
  → broadcast::Sender → SSE → sseService → realtimeStore(Map<node_id, readings>)
  → Dashboard (nodeReadings SSE patch)
  → ZoneDetail (useRealtimeStore merge)
```

### 天气面板
- TopBar 内嵌双行布局（row1 状态栏 + row2 天气信息）
- 数据源：和风天气免费套餐（5分钟轮询）
- 推荐策略：`/weather/now` + `/weather/3d` + `/weather/24h`(代替minutely)
- `safe_proxy()` 处理免费套餐 403 → 200+空数据（minutely/warning）
- 天气刷新时间显示在 row1 左侧

### 完成页面
- Dashboard (SSE实时更新 + 健康评分 + 区域概览)
- 区域详情 (ZoneDetail, 实时数据流)
- 节点列表 (NodeList)
- 规则管理 (RuleList)
- 数据查询 (DataQuery, 多指标折线图)
- AI 决策 (AIDecisions)
- 系统设置 (Settings)
- 设备控制面板 (ControlPanel)

### 关键组件
| 组件 | 路径 | 用途 |
|------|------|------|
| `TopBar` | `components/Layout/` | 天气面板+ SSE状态 |
| `HealthScoreBar` | `components/` | 健康评分进度条 + 趋势 |
| `EmergencyBanner` | `components/` | 紧急告警横幅 |
| `MetricRow` | `components/` | 单指标显示条（值/进度/状态） |
| `ControlPanel` | `components/` | 开关/卷膜器控制（发 `params`） |
| `LineChart` | `components/Charts/` | ECharts 多指标折线图 |
| `dashboardStore` | `stores/` | 数据中枢（SSE+fetch+AI评估） |
| `realtimeStore` | `stores/` | SSE 实时读数缓存 |
| `echartsTheme` | `theme/` | 指标颜色/标签映射 |

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

# 真实 ESP32 串口桥接
python3 scripts/serial_bridge.py /dev/ttyUSB0
```

Dashboard: http://localhost:3001
Tailscale Funnel: https://zero-1.taile2b316.ts.net

## 已知坑点

- `rumqttc` 0.24 的 `AsyncClient` 和 `EventLoop` 要分开创建
- SQLite `enabled` 字段是 `INTEGER`，与 `bool` 比较用 `== 1i64`
- 命令轮询返回的 `id` 必须是字符串（ESP32 `const char*` 接收），否则崩溃
- `mosquitto` 子进程启动后即退出（不影响 MQTT 功能，但需排查）
- LLD 链接器与工具链版本可能不兼容，`cargo clean` 后用 `CARGO_BUILD_JOBS=1` 可缓解内存不足
- 内存仅 1.8GB，并行 rustc 进程容易 OOM，编译需单线程或低并行度
- `ObsidianKnowledge::safe_path()` 需要 vault 路径存在才能 canonicalize，否则回退到字符串过滤
- 前端 metric 名须与 DB 一致（`temperature` vs `air_temp`），`dataApi.query()` 已不传 metric 参数，由前端 `useMemo` 过滤
- QWeather 免费套餐不支持 `/weather/minutely` 和 `/weather/warning`，后端 `safe_proxy()` 返回 200+空数据
- `dashboard/node-readings` 每次返回24h全量数据会 OOM，已修复为只返回最新值

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

## 实时遥测与 UI 完善（2026-05-22）

### 核心变更
| # | 变更 | 说明 |
|---|------|------|
| 1 | **SSE 事件携带 readings** | `telemetry.rs` 广播包含 `readings: [{metric,value,unit}]` |
| 2 | **dashboardStore SSE 订阅** | `fetchAll()` 启动 SSE 监听，`type:"telemetry"` 事件原地 patch `nodeReadings` |
| 3 | **TopBar 天气面板重写** | 双行布局：row1=时间+SSE状态，row2=实况+3天预报+逐时降水+预警 |
| 4 | **QWeather 免费套餐兼容** | `safe_proxy()` 返回 200+空数据代替 502；minutely 替换为 24h 接口 |
| 5 | **ZoneDetail 实时数据** | 订阅 `useRealtimeStore.readings`，SSE 推送时自动合并最新值 |
| 6 | **DataQuery 指标名修正** | `air_temp→temperature` 等与 DB 对齐，图表不再空白 |
| 7 | **DataQuery 折线图切换** | `key` + `notMerge: true`，取消勾选时正确隐藏折线 |
| 8 | **echartsTheme 独立** | 5 种不同颜色映射，metric 标签集中管理 |
| 9 | **命令 422 修复** | `deviceApi.sendCommand` 发送 `params` 字段（非 `payload`），与后端 `CommandPayload` 对齐 |
| 10 | **模拟器彻底清除** | `simulate_http.py`/`simulate_node.py` 删除，数据库 ~3万条模拟数据清空 |

### 变更文件清单
```
新增: agri-ui/src/theme/echartsTheme.ts     # 指标颜色/标签 ECharts 主题
新增: API-INTEGRATION-PLAN.md               # 第三方 API 集成评估
修改: agri-core/src/telemetry.rs            # SSE 广播增加 readings 数组
修改: agri-server/src/weather.rs            # safe_proxy() 免费套餐容错
修改: agri-ui/src/stores/dashboardStore.ts  # SSE 订阅 + healthTrend
修改: agri-ui/src/stores/realtimeStore.ts   # 实时读数缓存 (Map<nodeId, readings>)
修改: agri-ui/src/components/Layout/TopBar.tsx/.css  # 天气面板完整重写
修改: agri-ui/src/pages/ZoneDetail/ZoneDetail.tsx    # 实时数据流接入
修改: agri-ui/src/pages/DataQuery/DataQuery.tsx      # 指标名修复 + 图表切换
修改: agri-ui/src/components/Charts/LineChart.tsx    # notMerge 模式
修改: agri-ui/src/components/ControlPanel/ControlPanel.tsx  # params 字段名
修改: agri-ui/src/services/api.ts             # dataApi.query() 端点修正 + params 字段
删除: agri-ui/src/services/weather.ts         # 旧版 HeWeather 服务
删除: agri-ui/src/components/WeatherPanel/    # 废弃天气组件
删除: agri-ui/src/pages/ZoneList/             # 废弃区域列表页面
删除: scripts/simulate_http.py                # HTTP 模拟器
删除: scripts/simulate_node.py                # MQTT 模拟器
删除: agri-server/static/assets/旧bundle      # 旧构建产物
```

## 内存泄漏与稳定性修复（2026-06-02）

### 后端内存泄漏

| # | 问题 | 修复 | 文件 |
|---|------|------|------|
| 1 | **`dashboard/node-readings` 全量加载 24h 数据** — 单传感器每天 ~5万行，全部加载到内存再序列化，前端只取最后一条 | 改用 `MAX(id)` 子查询只返回每个 metric 最新值 | `routes.rs` |
| 2 | **`dashboard/area-readings` 无 LIMIT** — 加载 crop batch 全周期 sensor_readings | 加 `LIMIT 1000` | `routes.rs` |
| 3 | **`devices/:id/readings` 无默认 LIMIT** — 不传 limit 时返回全表 | 默认 `LIMIT 100`，max 5000 | `routes.rs` |
| 4 | **`reqwest::get()` 每次新建 HTTP Client** — 泄漏连接池 | 全局 `OnceLock<Client>` 复用 | `weather.rs` |
| 5 | **无设备离线检测** — telemetry 设 `online` 但永不设 `offline` | 规则引擎每 30 秒标记超 5 分钟设备为 offline | `rule_engine.rs` |

### ESP32 固件稳定性（v2.1.1）

| # | 问题 | 修复 |
|---|------|------|
| 1 | **`String` 堆碎片** — HTTP 函数每次调用分配多个 `String`（url/resp/body） | 全部改用 `char[]` 栈缓冲区 + `snprintf` + 流式读取 |
| 2 | **TWDT 复位** — `readSoilSensor` 忙等待循环不 yield，土壤无响应时跑满 1 秒触发 WatchDog | 所有 busy-wait 循环加 `delay(1)` |
| 3 | **WiFi 失败永久深度休眠** — `ESP.deepSleep(0)` 需硬件复位才唤醒 | 改为 3 次重试 + `ESP.restart()` |
| 4 | **TLS 内存泄漏** — `WiFiClientSecure` 长期不重建 | 每 100 次 HTTP 重建 client |
| 5 | **JSON 栈溢出风险** — `serializeJson(doc, buf)` 无边界检查 | 加 `serializeJson(doc, json, sizeof(json))` + 返回值校验 |
| 6 | **WatchDog 无喂狗** — `loop()` 没有主动喂狗 | `esp_task_wdt_reset()` |
| 7 | **命令 `id` 空指针** — 后端返回非字符串 id 时崩溃 | 加 `if (!id) continue` |
| 8 | **诊断残留 `String`** — `diagnoseRS485()` 用 `readString()` | 改为 `char[]` 手动读取 |

### 变更文件清单
```
修改: agri-server/src/routes.rs           # 全量历史→最新值, 加 LIMIT
修改: agri-server/src/rule_engine.rs      # 离线检测 + WatchDog
修改: agri-server/src/weather.rs          # OnceLock<Client> 复用
修改: agri-ui/src/stores/dashboardStore.ts # 适配 latest 字段
修改: esp32-firmware/src/main.ino         # v2.1.1: 无String + 喂狗 + TLS重建 + 重试
```

## 离线缓冲区（2026-06-03）

### LittleFS 环形缓冲区（方案 A，已实施）
- ESP32 将失败上报的 JSON 追加到 `/buffer.dat`（LittleFS）
- 间隔 10s 采集，缓冲区 2000 行 ≈ 768KB Flash，覆盖 ~5.5 小时
- `publishTelemetry` 成功后触发 `flushBuffer()`，每次回放最多 20 条
- `trimBufferTail()` 在缓冲区超限时截断，保留最新 2000 行
- 服务器新增 `POST /api/v1/telemetry/batch` 端点，支持批量补录
- 与兄弟节点推断互补：单节点离线靠推断，服务器故障靠缓存

### 变更文件清单
```
修改: esp32-firmware/src/main.ino     # LittleFS 缓冲 + 回放
修改: agri-server/src/routes.rs       # /telemetry/batch 批量端点
文档: AGENTS.md                        # MQTT 下一版本规划
```

## 下一版本计划：MQTT 解耦（v2.0）

### 目标
将 rumqttd broker 从 agri-server 解耦为独立进程，ESP32 固件从 HTTP 迁移到 MQTT，利用 MQTT QoS 保证消息不丢失。

### 设计
```
ESP32 → MQTT (QoS 1) → rumqttd (独立进程) → agri-server (consumer)
                                                ↓
                                             SQLite + SSE
```

| 组件 | 职责 |
|------|------|
| rumqttd | 独立 broker 进程，QoS 1 持久化暂存 |
| agri-mqtt (consumer) | 订阅 `telemetry/+/+`，转发到 `process_telemetry()` |
| agri-server | 不变，HTTP 和 MQTT 两条入口最终汇聚到 `process_telemetry` |

### 优点
- MQTT 原生 QoS 保证，broker 挂起不影响 publisher
- 减少 HTTP 连接开销（每次 TLS 握手）
- broker 可做消息桥接、离线缓存

### 依赖
- **ESP32 固件重写**为 MQTT publisher（替换现有 HTTP 逻辑）
- **rumqttd 配置持久化**：`persistence.clean_session = false`
- **consumer 幂等**：MQTT 可能重复投递，需 `(node_id, metric, timestamp)` 作为 dedup key

### 优先级
低 — 当前 HTTP + LittleFS 方案已覆盖服务器故障场景，MQTT 解耦为远期架构优化。
```
