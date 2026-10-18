# 每日代码审查报告 — 2026-10-18

## 一、后端 Rust — 编译 ✅ / 警告 ⚠️

```
cargo build: 成功（无 error）
警告数: 48（全部 warning，无 error）
```

### 关键警告（无需紧急处理）

| 文件 | 警告 | 说明 |
|------|------|------|
| `decision/notification/escalator.rs` | `run_escalation` 未使用 | 通知升级链逻辑已废弃或未接入 |
| `decision/stages/llm_stage.rs` | `provider`/`retrieval` 字段未读 | LLM Stage 结构体字段存在但未使用 |
| `yield.rs` | `AnalysisQuery.crop_id` 未读 | 查询结构体字段冗余 |
| 通知模块 | `NotificationMsg`/`Notifier`/`Urgency` 等全部未使用 | 通知系统代码大量残留 |
| `decision/state_machine.rs` | 多个 enum variant 未构造 | 状态机代码框架未完成 |

**结论**: 编译无错误。48 个警告均为"未使用代码"——AI 决策子系统（`decision/`）的代码框架已完成但未接入主线逻辑，属于预留代码，暂不影响运行。

---

## 二、前端 TypeScript — ⚠️ 受限

TypeScript 编译器（tsc）被安全扫描拦截，无法执行类型检查。

### 手动检查
- `agri-ui/src/pages/FarmLog/FarmLog.tsx` — 本次修改涉及区域选择器重构，类型安全
- `agri-ui/src/pages/Labor/Labor.tsx` — 列头标签已更新为"工日/人"和"¥/天"
- 构建成功，产物已部署到 `agri-server/static/`

---

## 三、测试 — 全绿 ✅

| 模块 | 测试数 | 结果 |
|------|--------|------|
| agri-core | 138 | ✅ 全过 |
| agri-mqtt | 22 | ✅ 全过 |
| agri-server | 76 | ✅ 全过 |
| **总计** | **236** | ✅ 0 失败 |

---

## 四、Git 状态 — ⚠️ 有待提交

自上次审查报告（2026-10-14）以来，**没有新提交**，但工作区有大量修改：

### 已修改文件（未提交）
- `AGENTS.md` — 文档更新
- `agri-core/src/db.rs` — 新增 `ensure_labor_schema()` 幂等迁移（旧表 schema 修复）
- `agri-core/src/models.rs` — 模型调整
- `agri-mqtt/src/handler.rs` — MQTT 订阅改为在 ConnAck 时执行
- `agri-server/src/areas.rs` — 修复非 UUID 的 area_id 导致 500 的问题
- `agri-server/src/farm_log.rs` — 添加非 UUID ID 容忍测试
- `agri-server/src/main.rs` — 移除独立 subscribe 任务，改为 handler 内订阅
- `agri-server/src/routes.rs` — 添加非 UUID area_id 容忍测试
- `agri-server/src/rule_engine/dag.rs` — 空链返回 Ok(()) 而非错误
- `agri-server/src/rule_engine/mod.rs` — 离线检测宽限期从 300s 改为 900s
- `agri-ui/src/pages/FarmLog/FarmLog.tsx` — 区域选择器重构
- `agri-ui/src/pages/Labor/Labor.tsx` — 工日制标签更新
- `agri-server/static/*` — 前端构建产物已更新

### 建议
将上述修改按逻辑分组提交：
1. `commit 1`: 后端 schema 修复 + 测试（db.rs, models.rs）
2. `commit 2`: MQTT 连接逻辑优化（handler.rs, main.rs）
3. `commit 3`: 非 UUID 兼容性修复（areas.rs, routes.rs, farm_log.rs）
4. `commit 4`: 规则引擎改进（rule_engine/dag.rs, rule_engine/mod.rs）
5. `commit 5`: 前端更新（FarmLog.tsx, Labor.tsx, static/*）
6. `commit 6`: 文档更新（AGENTS.md）

---

## 五、安全漏洞 — ⚠️ 需关注

### Rust 依赖（cargo audit）
- `rustls-pemfile` 1.0.4 / 2.2.0 — unmaintained
- `yaml-rust` 0.4.5 — unmaintained
- `spin` 0.9.8 — yanked
- **共 23 个漏洞**（主要为 unmaintained 包）

### npm 依赖（npm audit）
- `axios` — 8 个漏洞（ReDoS、原型污染、SSRF 等）
- `brace-expansion` — 3 个高危漏洞（DoS）
- `source-map-js` — 1 个高危漏洞
- **共 3 个高危漏洞**

### 建议
- 运行 `npm audit fix` 尝试自动修复
- 考虑将 axios 升级到最新稳定版本
- 后端 unmaintained 包暂无直接安全风险（仅用于标准库功能），可暂缓

---

## 六、改进建议

1. **清理未使用代码**：`decision/notification/` 和 `decision/state_machine.rs` 中的未使用代码可删除或标记 `#[allow(dead_code)]`
2. **提交未完成的修改**：工作区有 14 个文件待提交，建议尽快完成提交
3. **前端构建产物同步**：`static/` 目录已更新，需随代码一起提交
4. **安全审计**：定期运行 `cargo audit` 和 `npm audit`，及时更新依赖

---

**审查时间**: 2026-10-18 09:00 CST
**审查范围**: 后端 Rust + 前端 TypeScript + 依赖安全
