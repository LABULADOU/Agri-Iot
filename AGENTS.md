# Agri-iot 项目笔记

## 项目概述
农业物联网监控系统，Rust 技术栈：
- **agri-core**: 核心库（模型、DB、错误定义）
- **agri-server**: 后端服务（Axum + SQLx）
- **agri-mqtt**: MQTT 通信（rumqttd broker + rumqttc client）
- **agri-ui**: 前端（React + TypeScript + Vite + Ant Design）
- **esp32-firmware**: ESP32 固件（Arduino）

## 后端整改完成（2026-05-06）

### 已修复问题
1. **SQL 注入风险** → `routes.rs` 改用参数化查询
2. **MQTT 功能未启用** → `main.rs` 启动 broker/client，`handler.rs` 监听消息
3. **中间件未生效** → 添加 `.layer(middleware::from_fn(...))`
4. **错误处理不统一** → `AppError` 增加 `RuleNotFound`，实现 `IntoResponse` 转换
5. **规则引擎时间检查粗糙** → 改为精确秒级判断
6. **MQTT topic 不一致** → 模拟器 topic 与 handler 匹配
7. **rumqttc API 不匹配** → 使用正确的 `AsyncClient + EventLoop` 模式
8. **中间件模块冲突** → 重命名 `middleware.rs` 为 `request_logger.rs`

### 编译状态（2026-05-13 最新）
- ✅ `cargo check` 全项目通过
- 构建产物：`target/debug/agri-server`
- MQTT：支持云 Broker（通过 `MQTT_HOST` 配置，默认 `broker.emqx.io`），非本地地址时自动跳过内置 Broker

## 后端优化进度

### 响应 JSON 序列化（未完成）
- 所有 API 当前使用手动 `serde_json::json!()` 构建 JSON
- 枚举（`DeviceType`、`DeviceStatus`、`TriggerType`）未实现 `sqlx::Type`，无法直接 `FromRow`
- 后续建议：为枚举实现 sqlx 支持，或保持手动构建

### 单元测试（已有 54 个测试）
- `agri-core`：模型 9 个 + 错误处理 9 个 = 18 个
- `agri-mqtt`：消息处理 15 个 + 集成 7 个 = 22 个
- `agri-server`：路由 11 个 + 规则引擎 3 个 = 14 个

### 配置管理统一（已完成）
- `config/` 已移除，统一使用 `.env` 加载配置
- `agri-core/src/models.rs`：移除死代码 `AggregatedReading`、`CommandLog`、`CommandStatus`、`SensorUtils`、`FromRow` 派生
- `agri-core/migrations/001_init.sql`：替为 `migrations/` 的符号链接（消除重复）
- `agri-server/src/routes.rs`：`AggregatedQuery.metric` 接入实际查询（原为死字段）
- `deploy/verify.sh`：修复 `middleware.rs` → `request_logger.rs` 过时引用

## 前端（React + Vite，已完成迁移）
- 新 UI 替换旧 Yew WASM 前端（2026-05-13）
- 旧 `agri-frontend/` 已移除
- 构建输出到 `agri-server/static/`，由后端 fallback 服务托管
- 技术栈：React 19 + Ant Design 6 + ECharts + Zustand + React Router 7

## ESP32（更新于 2026-05-13）
- 固件：仅采集 DHT22 温湿度，上报至云 MQTT Broker（`broker.emqx.io`）
- 使用前需修改 `src/main.ino` 中的 WiFi 凭据（`WIFI_SSID` / `WIFI_PASSWORD`）
- 冗余传感器（土壤湿度、光照）和控制指令保留代码已清理
- 根目录 `main.ino` 重复文件已移除，`src/main.ino` 为唯一源文件

## 关键文件位置
- 路由：`agri-server/src/routes.rs`（返回 `Response`，错误处理用 `app_error_to_response`）
- 状态管理：`agri-server/src/state.rs`（持有 `pool`、`mqtt_client`、`rules_cache`）
- 规则引擎：`agri-server/src/rule_engine.rs`（5秒轮询 + 每分钟刷新缓存）
- MQTT 处理：`agri-mqtt/src/handler.rs`（监听 `agri/node/+/telemetry` 和 `agri/node/+/status`）
- 数据库迁移：`migrations/001_init.sql`
- 请求日志：`agri-server/src/request_logger.rs`（原 `middleware.rs`）
- 前端页面：`agri-ui/src/pages/`
- 前端组件：`agri-ui/src/components/`
- 前端 API：`agri-ui/src/services/api.ts`
- 前端状态：`agri-ui/src/stores/`

## 启动方式
```bash
# 1. 构建前端（首次或修改后）
cd agri-ui && npm install && npm run build && cd ..

# 2. 构建后端
cargo build

# 3. 启动后端
./target/debug/agri-server

# 4. 模拟传感器数据（新终端）
python3 scripts/simulate_node.py
```

## 下次上手快速指南

### 1. 验证当前状态
```bash
cd /home/admino/Agri-iot
cargo check  # 应该全项目通过
```

### 2. 接着做优化（建议顺序）
1. **配置管理统一**：已完成（使用 `.env`）
2. **单元测试**：已有 54 个，可继续增加路由集成测试
3. **JSON 序列化**：解决枚举的 sqlx 支持问题，或为枚举实现 `FromRow`

### 3. 前端开发
```bash
cd agri-ui
npm install
npm run dev    # 开发模式，端口 3001，API 代理到后端 3000
npm run build  # 构建到 agri-server/static/
```

## 已知坑点
- `rumqttc` 0.24 的 `AsyncClient` 和 `EventLoop` 要分开创建（`AsyncClient::new` 返回 `(AsyncClient, EventLoop)`）
- `axum` 的路由函数返回 `Response` 比返回 `Result<T, AppError>` 更简单（`AppError` 已实现 `IntoResponse`）
- SQLite 的 `enabled` 字段是 `INTEGER`，读取时是 `i64`，与 `bool` 比较要用 `== 1i64`
- 时间戳：数据库存 `i64`（Unix 秒），模型用 `DateTime<Utc>`，API 层需要转换

## 项目记忆
- 本文档（`AGENTS.md`）是项目记忆，每次新对话先看这个
- 后端核心功能已完善，优先做优化和测试
- 前端已迁移到 React（agri-ui），旧 Yew 前端已移除
- ESP32 按用户要求暂不处理
