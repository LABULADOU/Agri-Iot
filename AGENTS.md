# Agri-iot 项目笔记

## 项目概述
农业物联网监控系统，Rust 技术栈：
- **agri-core**: 核心库（模型、DB、错误定义）
- **agri-server**: 后端服务（Axum + SQLx）
- **agri-mqtt**: MQTT 通信（rumqttd broker + rumqttc client）
- **agri-frontend**: 前端（Yew WASM）
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

### 编译状态（2026-05-06 最新）
- ✅ `cargo check` 全项目通过
- 构建产物：`target/debug/agri-server`

## 后端优化进度

### 响应 JSON 序列化（未完成）
- **尝试过**：为 `Device`、`SensorReading`、`Rule` 添加 `sqlx::FromRow` 派生
- **遇到问题**：`DeviceType`、`DeviceStatus`、`TriggerType` 枚举需要实现 `sqlx::Type` + `sqlx::Decode`，否则 `query_as` 报错
- **当前方案**：回退到手动构建 JSON（`serde_json::json!`），编译通过
- **后续建议**：要么为枚举实现 sqlx 支持，要么继续用手动 JSON 构建

### 单元测试（未开始）
- [ ] 为 `agri-core` 模型添加测试
- [ ] 为 `agri-server` 路由添加集成测试
- [ ] 为规则引擎添加逻辑测试

### 配置管理统一（未开始）
- **现状**：`.env`（DATABASE_URL、SERVER_PORT、MQTT_BROKER_PORT、RUST_LOG）和 `config/default.toml`（server、database、mqtt、logging）并存
- **建议**：二选一，推荐用 `config` crate 统一加载 `config/default.toml`，敏感信息放 `.env` 或环境变量

## 前端（优先级低，未开始）
- [ ] 告警记录页面（`alerts.rs` 仅占位符）
- [ ] 系统设置页面（`settings.rs` 仅占位符）
- [ ] 前端设备/规则创建表单
- [ ] 数据可视化图表

## ESP32
- [ ] 移除固件中的硬编码 WiFi 凭据（`main.ino:16-17`）
- **按用户要求：暂不处理**

## 关键文件位置
- 路由：`agri-server/src/routes.rs`（返回 `Response`，错误处理用 `app_error_to_response`）
- 状态管理：`agri-server/src/state.rs`（持有 `pool`、`mqtt_client`、`rules_cache`）
- 规则引擎：`agri-server/src/rule_engine.rs`（5秒轮询 + 每分钟刷新缓存）
- MQTT 处理：`agri-mqtt/src/handler.rs`（监听 `agri/node/+/telemetry` 和 `agri/node/+/status`）
- 数据库迁移：`migrations/001_init.sql`
- 请求日志：`agri-server/src/request_logger.rs`（原 `middleware.rs`）

## 启动方式
```bash
# 1. 构建（首次或代码修改后）
cargo build

# 2. 启动后端
./target/debug/agri-server

# 3. 模拟传感器数据（新终端）
python3 scripts/simulate_node.py
```

## 下次上手快速指南

### 1. 验证当前状态
```bash
cd /home/admino/Agri-iot
cargo check  # 应该全项目通过
```

### 2. 接着做优化（建议顺序）
1. **配置管理统一**：选 `.env` 或 `config.toml` 二选一，减少配置源
2. **单元测试**：从 `agri-core` 的模型开始，再到路由和规则引擎
3. **JSON 序列化**：解决枚举的 sqlx 支持问题，或为枚举实现 `FromRow`

### 3. 如果要做前端
- 先完善 `alerts.rs` 和 `settings.rs` 的占位符页面
- 添加设备/规则创建表单（需要后端先完善 API）
- 数据可视化（需要图表库，如 `plotters` 或前端图表库）

## 已知坑点
- `rumqttc` 0.24 的 `AsyncClient` 和 `EventLoop` 要分开创建（`AsyncClient::new` 返回 `(AsyncClient, EventLoop)`）
- `axum` 的路由函数返回 `Response` 比返回 `Result<T, AppError>` 更简单（`AppError` 已实现 `IntoResponse`）
- SQLite 的 `enabled` 字段是 `INTEGER`，读取时是 `i64`，与 `bool` 比较要用 `== 1i64`
- 时间戳：数据库存 `i64`（Unix 秒），模型用 `DateTime<Utc>`，API 层需要转换

## 项目记忆
- 本文档（`AGENTS.md`）是项目记忆，每次新对话先看这个
- 后端核心功能已完善，优先做优化和测试
- 前端优先级低，ESP32 按用户要求暂不处理
