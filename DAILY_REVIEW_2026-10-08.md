# 每日代码审查报告 — 2026-10-08

## 一、后端 Rust 编译状态

✅ **编译通过**，无错误。共 **48 个 warning**。

### Clippy 警告分类

| 类别 | 数量 | 说明 |
|------|------|------|
| 未使用函数/方法 | ~25 | AI 决策系统模块（`DecisionEngine`、`FlowContext`、`ApprovalGate`、`NotificationDispatch` 等）尚未接入主流程 |
| 未使用字段 | ~15 | `LlmStage.provider`/`retrieval`、`AnalysisQuery.crop_id`、各类状态字段 |
| 未使用枚举变体 | ~8 | `PrecipitationIntensity`、`WeatherCondition`、`VentilationState` 等枚举的变体 |

> **说明**：这些警告主要来自 AI 决策系统重构（v3.0+）中新建但尚未完全接入的模块，属于开发中的代码，非 bug。

### 重点警告
- `agri-server/src/decision/notification/escalator.rs:56` — `run_escalation` 未使用
- `agri-server/src/yield.rs:248` — `AnalysisQuery.crop_id` 字段未使用

---

## 二、前端 TypeScript 状态

✅ **无类型错误**（`tsc --noEmit` 通过）

---

## 三、Git 近期提交

近 3 天仅有 2 条自动提交的审查报告，**无新代码提交**。最近功能提交为 **2026-09-20** 的农事出库联动 + 用工成本模块。

```
61745e0 docs: 每日代码审查报告 2026-10-07
d961bda docs: 每日代码审查报告 2026-10-06
e74e415 docs: 每日代码审查报告 2026-10-04
6144722 docs: 每日代码审查报告 2026-10-04
b1cd1b0 fix: 修复 anomaly.rs SQL 查询未 await 问题 + 每日代码审查报告
f923b6d docs: AGENTS.md 记录农事出库联动 + 用工成本模块
```

---

## 四、依赖安全审计

### 🔴 Rust — cargo audit：11 个漏洞，5 个警告

| 严重性 | 依赖包 | 版本 | 问题 | 修复方案 |
|--------|--------|------|------|----------|
| **高危** | `quinn-proto` | 0.11.14 | 远程内存耗尽（QoS-1 拒绝服务） | 升级到 >=0.11.15 |
| **高危** | `crossbeam-epoch` | 0.9.18 | 无效指针解引用 | 升级到 >=0.9.20 |
| **高危** | `sqlx` | 0.7.4 | 二进制协议误解释 | 升级到 >=0.8.1（**破坏性升级**） |
| **高危** | `rustls-webpki` | 0.101.7/0.102.8 | 证书名称约束绕过 + CRL 解析 panic（×4条） | 升级到 >=0.103.12 或 >=0.103.13 |
| **中危** | `rsa` | 0.9.10 | Marvin 时序侧信道攻击 | **无修复方案**，如不使用 RSA 加密可忽略 |
| 警告 | `paste` | 1.0.15 | 不再维护 | 低风险 |
| 警告 | `rustls-pemfile` | 1.0.4/2.2.0 | 不再维护 | 低风险 |
| 警告 | `yaml-rust` | 0.4.5 | 不再维护 | 低风险 |
| 警告 | `spin` | 0.9.8 | 已 yanked | 低风险 |

> **优先处理**：`quinn-proto`、`crossbeam-epoch`、`rustls-webpki` 均可安全升级；`sqlx` 升级需谨慎评估。

### 🔴 Node.js — npm audit：3 个高危漏洞

| 依赖 | 问题 | 修复 |
|------|------|------|
| `axios@1.16.0` | 12 项漏洞（原型链污染、ReDoS、SSRF、代理绕过等） | `npm audit fix` 升级到最新 |
| `brace-expansion` | 3 项 DoS（二次时间复杂度/栈溢出） | `npm audit fix` |
| `source-map-js` | 1 项 DoS（event-loop 阻塞） | `npm audit fix` |

> **优先处理**：`npm audit fix` 可直接修复全部 3 项。axios 升级注意 API 兼容性。

---

## 五、测试状态

| 模块 | 测试数 | 状态 |
|------|--------|------|
| agri-core | 136 | ✅ 全过 |
| agri-server | 72 | ✅ 全过 |
| agri-mqtt | 22 | ✅ 全过 |
| **总计** | **230** | ✅ 全过 |

---

## 六、改进建议（按优先级）

### P0 — 安全漏洞（需尽快处理）
1. **Rust**: 升级 `quinn-proto` → `>=0.11.15`、`crossbeam-epoch` → `>=0.9.20`、`rustls-webpki` → `>=0.103.12`
2. **Node.js**: 执行 `npm audit fix` 修复 axios/brace-expansion/source-map-js
3. **sqlx**: 评估升级到 0.8.x（破坏性变更，需测试验证）

### P1 — 代码质量
4. **清理 AI 决策模块未使用代码**：`DecisionEngine`、`FlowContext`、`ApprovalGate`、`NotificationDispatch` 等若暂不使用，可加 `#[allow(dead_code)]` 或暂存分支
5. **`AnalysisQuery.crop_id`** 字段未使用，可移除或补充用途

### P2 — 常规维护
6. 更新未维护依赖（`paste`、`rustls-pemfile`、`yaml-rust`）— 低风险，择机处理