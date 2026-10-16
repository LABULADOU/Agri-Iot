# 每日代码审查报告 — 2026-10-13

## 概览

| 项目 | 状态 |
|------|------|
| 后端编译 | ✅ 通过（48 warnings，无 error） |
| 测试 | ✅ 234/234 通过 |
| TypeScript | ✅ 无错误 |
| ESLint | ✅ 无错误 |
| 安全漏洞 | ⚠️ 见下文 |
| 最近 commit | `2959bf0 docs: 每日代码审查报告 2026-10-13` |

---

## 一、后端 Rust

### 编译警告（48 条，均为 dead code）

警告均来自**未完成/未接入的决策引擎框架**和少数辅助函数：

| 类别 | 警告数 | 说明 |
|------|--------|------|
| `agri-server/src/decision/` | ~35 | DecisionEngine、FlowContext、StageOut、RainState/WindState 状态机、NotificationDispatch、ShiftRouter、EscalationChain 等全部为骨架代码，标有 `// TODO(decision)` |
| `yield.rs:248` | 1 | `AnalysisQuery.crop_id` 字段未读取 |
| `rule_engine/mod.rs:303` | 1 | `evaluate_condition_rule` 未被调用 |
| `ai_routes.rs:573` | 1 | `ObsidianListQuery.filter_type` 字段未使用 |
| `rule_engine/nodes.rs` | 4 | 4 个节点结构体的 `id` 字段定义后未读 |
| `mqtt_test` 示例 | 1 | unused import |

**评估**：`decision/` 模块是已设计完整但未上线的预留框架（紧急保护 Tier 1、状态机 Tier 2、LLM 评估 Tier 3），当前 `start()` 只创建了实例后用 `let _` 丢弃。建议加 `#[allow(dead_code)]` 消除噪音，待调度器实现后自然消亡。

### 测试：234 通过，0 失败
- agri-core: 138（含 labor schema 迁移测试）
- agri-mqtt: 22
- agri-server: 74

---

## 二、前端 TypeScript

`tsc --noEmit` + ESLint 输出为空，**零错误**。

---

## 三、安全漏洞

### Rust 依赖（cargo audit 网络失败）
无法连接 GitHub Advisory DB，跳过。已知低风险项：rustls-pemfile / yaml-rust / spin 等 unmaintained/yanked 包，内部工具环境风险可控。

### NPM 依赖（npm audit，3 个高危）

| 包 | 范围 | 问题 | 风险 |
|----|------|------|------|
| axios | 1.0.0–1.19.0 | ReDoS + 9 处原型链污染/Header 注入/SSRF bypass | 低（仅前端 SPA，非对外暴露服务）|
| brace-expansion | 4.0.0–5.0.11 | DoS（栈溢出）| 低（构建依赖）|
| source-map-js | 1.0.0–1.2.1 | DoS（indexed offset）| 低（构建依赖）|

**建议**：`npm audit fix` 一次即可，非紧急。

---

## 四、未提交变更分析

```
M  agri-core/src/db.rs              # ensure_labor_schema() 迁移逻辑 + 2 测试
M  agri-mqtt/src/handler.rs         # start_listener 新增 sub_client 参数
M  agri-server/src/main.rs          # 移除内联 subscribe 循环，改用 handler 内部订阅
M  agri-server/src/areas.rs         # list_areas/get_area 改用元组解析非 UUID ID + 2 测试
M  scripts/init.sh                  # 日志轮转（>200MB 截断保留 20MB）
M  agri-ui/src/pages/Labor/Labor.tsx # 工日/人 + ¥/天 标签修正
M  agri-server/static/index.html    # 前端 rebuild
??   agri-server/static/assets/*.js (新 bundle)
```

| 文件 | 评估 |
|------|------|
| `db.rs` | ✅ 正确。旧版 `labor_records` 有 `team`/`paid_status INTEGER`，与当前 `worker`/`paid_status TEXT` 不兼容，幂等重建并迁移数据，测试覆盖 |
| `handler.rs` + `main.rs` | ✅ 改进。原 `create_mqtt_client` 中每 10s 轮询订阅的循环被移除，改为在 `Event::ConnAck` 回调中一次性订阅，更可靠；`listener_sub_client` 传入 handler 管理 |
| `areas.rs` | ✅ 修复。seed 数据含 `'area-demo'` 非 UUID 文本 ID，强类型反序列化触发 500，前端 Promise.all 连带失败；改用元组解析后行为正常，含回归测试 |
| `init.sh` | ✅ 实用。日志自动轮转，避免磁盘撑满 |
| `Labor.tsx` | ✅ 已对齐 AGENTS.md 口径澄清 |

---

## 五、建议行动项

| 优先级 | 事项 | 文件 |
|--------|------|------|
| P0 | 提交当前 6 处变更 + 静态资源 | — |
| P1 | decision/ 模块加 `#[allow(dead_code)]` 消除 35 条噪音 | `decision/**/*.rs` |
| P2 | `npm audit fix` 修复 3 个高危漏洞 | `agri-ui/package.json` |
| P3 | 清理 yield.rs `crop_id` 未读字段、rule_engine `evaluate_condition_rule` 未调用函数 | `yield.rs`, `rule_engine/mod.rs` |

---

## 六、结论

- ✅ 无编译错误，无测试失败
- ✅ 无 TypeScript/ESLint 错误
- ✅ 未提交变更均为修复性质，无回归风险
- ⚠️ 注意提交当前工作区变更（尤其 areas.rs 修复是非 UUID ID 的线上 500 bug）
- ⚠️ decision/ 框架代码长期 dead_code，建议标注或清理
