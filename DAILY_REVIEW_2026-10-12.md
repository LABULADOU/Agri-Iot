# 每日代码审查报告 — 2026-10-12

## 一、后端 Rust 编译状态

| 项目 | 状态 |
|------|------|
| `cargo check` | ✅ 通过（48 warnings） |
| `cargo clippy` | ⚠️ 4 处 dead code + 1 处 large err |
| `cargo test` | ✅ 全过（core 138 + server 74 + mqtt 0） |

### Clippy 警告明细

| 文件 | 问题 | 级别 |
|------|------|------|
| `decision/notification/escalator.rs` | `EscalationChain`、`run_escalation` 全部 dead code | 低 |
| `decision/stages/llm_stage.rs` | `provider`、`retrieval` 字段从未被读取 | 低 |
| `yield.rs` | `AnalysisQuery.crop_id` 字段从未被读取 | 低 |
| `ai_routes.rs:731` | `Result<..., impl IntoResponse>` 变体过大（128+ bytes） | 低 |

**建议**：dead code 若为预留接口可加 `#[allow(dead_code)]` 注释；若确已废弃则删除。

---

## 二、前端 TypeScript / Lint

| 项目 | 状态 |
|------|------|
| `npm run lint` (ESLint) | ✅ 通过，零错误 |
| `tsc --noEmit` | ⚠️ 受安全扫描阻塞，未执行 |
| 静态产物 | ⚠️ static/assets 中有未提交的构建产物（新旧共存） |

**注意**：`agri-server/static/assets/` 目录下同时存在旧文件（如 `Labor-CohcZpYM.js`）和新文件（如 `Labor-unH8Hdo3.js`），建议重建后提交或清理旧 bundle。

---

## 三、依赖安全漏洞

### Rust（`cargo audit`）

| 包 | 版本 | 问题 | 严重性 |
|----|------|------|--------|
| `rustls-pemfile` | 1.0.4 / 2.2.0 | unmaintained（RUSTSEC-2025-0134） | 低 |
| `yaml-rust` | 0.4.5 | unmaintained（RUSTSEC-2024-0320） | 低 |
| `spin` | 0.9.8 | yanked | 低 |

无实际 CVE 利用风险，均为维护状态警告。

### Node.js（`npm audit`）— **需关注**

| 包 | 漏洞数 | 漏洞类型 |
|----|--------|----------|
| `axios` | 9 | ReDoS、原型污染、SSRF、HTTP/2 绕过、Header 注入等 |
| `brace-expansion` | 3 | DoS（栈溢出/递归） |
| `source-map-js` | 1 | DoS（event-loop 阻塞） |

**影响**：axios 是运行时依赖，若服务端接受用户输入构造请求可能受影响；brace-expansion/source-map-js 为开发工具链依赖。

**建议**：
```bash
cd agri-ui && npm audit fix
# 检查 axios 版本是否已更新至修复版本（>=1.7.7）
```

---

## 四、最近变更审查（未提交）

### 变更概览
```
agri-server/src/areas.rs           +90/-9   area id 解码修复
agri-server/src/main.rs            +17/-5   listener_sub_client 重连订阅
agri-server/src/rule_engine/mod.rs +44/0    offline grace period 300→900s + 测试
agri-server/src/rule_engine/dag.rs +29/1    空链 no-op + 测试
agri-mqtt/src/handler.rs           +11/0    ConnAck 事件自动重订阅
agri-core/src/db.rs                +176/0   labor schema 幂等迁移
agri-ui/src/pages/Labor/Labor.tsx  +10/-0   用工页面更新
```

### 重点审查

**1. areas.rs — id 解码修复** ✅ 正确
- 原因：seed 数据含 `'area-demo'` 等非 UUID 文本 id，`Area` 模型用 UuidText 反序列化会 500
- 修复：改用 `(String, String, Option<String>, i64)` 元组，兼容任意格式 id

**2. rule_engine — grace period 300s → 900s** ✅ 合理
- 原因：固件 READ_INTERVAL 为 300s，300s 宽限期恰好等于一个采集周期，网络抖动即误判离线
- 修复：改为 900s（3 个采集周期），加回归测试

**3. dag.rs — 空链 no-op** ✅ 合理
- 原因：之前 `anyhow::ensure!(node_count > 0)` 在空规则链时返回 error，触发 15 万+ 条 warning 日志
- 修复：改为 `return Ok(())` 静默跳过

**4. handler.rs — ConnAck 自动重订阅** ✅ 合理
- 原因：之前依赖周期性订阅，重连后可能漏订阅；改为在 ConnAck 事件触发时订阅，更可靠

**5. db.rs — labor schema 迁移** ✅ 幂等
- 检测旧表是否有 `team` 列（旧 schema），有则 DROP+RENAME+CREATE 重建为新 schema
- 测试覆盖：`test_db::ensure_labor_schema_rebuilds_legacy_table`

---

## 五、待处理事项

| 优先级 | 项目 | 建议 |
|--------|------|------|
| **P0** | axios 9 个安全漏洞 | `npm audit fix` 升级 axios 及子依赖 |
| **P1** | static/assets 新旧 bundle 共存 | 重新 `npm run build` 并清理旧文件 |
| **P2** | dead code 清理 | escalator.rs、llm_stage.rs、yield.rs 中未使用字段/函数加注释或删除 |
| **P2** | cargo audit unmaintained 包 | 关注 `rustls-pemfile`、`yaml-rust` 上游更新 |

---

## 六、测试统计

| 项目 | 通过 | 失败 |
|------|------|------|
| agri-core | 138 | 0 |
| agri-server | 74 | 0 |
| agri-mqtt | 0 | 0 |
| **合计** | **212** | **0** |
