# 每日代码审查报告 — 2026-10-10

## 总体状态：🟢 健康

- **后端测试**: 232 全部通过 (agri-core 136 + agri-mqtt 22 + agri-server 74)
- **前端 TypeScript**: ✅ 无类型错误
- **编译**: ✅ 成功，零错误

---

## 🔴 高危 — 需立即处理

### npm 安全漏洞（axios）未修复

昨日报告已列出，本次运行 `npm audit fix` 被安全策略阻断（cron job 无法交互式确认）。

| 包 | 当前版本 | 修复版本 | 严重性 | CVE |
|---|---|---|---|---|
| `axios` | 1.19.0 | **1.20.0** | 🔴 高 | 原型链污染 + ReDoS + SSRF（多个 GHSA） |
| `brace-expansion` | 4.x-5.x | patch | 🔴 高 | DoS 栈溢出 |
| `source-map-js` | ≤1.2.1 | **1.2.2+** | 🔴 高 | DoS 事件循环阻塞 |

**修复命令**（需人工确认）：
```bash
cd /home/xuhan/Agri-Iot/agri-ui
npm audit fix    # 自动修复上述三项
```

axios 漏洞是**生产环境真实风险**——原型链污染可通过 HTTP 请求劫持/凭证泄露。该项目通过 Funnel 对外暴露，建议优先修复。

---

## 🟡 中危 — 建议本周处理

### 大量依赖过期（~18 个包）

| 包 | 当前 | 最新 | 影响 |
|---|---|---|---|
| `typescript` | 6.0.3 | **7.0.2** | 大版本跳跃，需回归测试 |
| `vite` | 8.1.3 | 8.3.4 | 低风险，直接更新 |
| `antd` | 6.3.7 | 6.6.5 | 低风险，直接更新 |
| `react` / `react-dom` | 19.2.6 | 19.3.0 | 低风险 |
| `dayjs` | 1.11.20 | 1.11.23 | 低风险 |
| `zustand` | 5.0.13 | 5.0.15 | 低风险 |
| `eslint` | 10.3.0 | 10.12.0 | 建议更新 |

**建议**: 先执行 `npm audit fix`，再逐个升级次要版本。TypeScript 6→7 是大版本跳跃，单独评估后再升级。

---

## 🟢 低危 — 代码质量

### 后端 dead code 警告（48 条）

主要集中在 `agri-server/src/decision/` 子目录：

```
agri-server/src/decision/
├── engine.rs      — DecisionEngine, FlowContext, StageOut 未使用
├── notification/  — EscalationChain, run_escalation 未使用
├── registry.rs    — RainState, WindState 枚举变体未构造
├── log.rs         — write_log, query_log 未使用
└── approval.rs    — ApprovalGate 未使用
```

**判断**: 这是 AI 决策引擎的**分阶段功能模块**，部分已实现但尚未接入主流程。不是 bug，但产生噪音。

**建议**: 
- 若近期计划启用：加 `#[allow(dead_code)]` 抑制警告，或完成集成
- 若已废弃：清理死代码

---

## 📝 Git 近期提交摘要（最近 5 次）

| 提交 | 说明 |
|---|---|
| `126dd0d` | docs: 每日代码审查报告 2026-10-09 |
| `00b4698` | docs: 每日代码审查报告 2026-10-08 |
| `61745e0` | docs: 每日代码审查报告 2026-10-07 |
| `d961bda` | docs: 每日代码审查报告 2026-10-06 |
| `b1cd1b0` | fix: 修复 anomaly.rs SQL 查询未 await 问题 |

**观察**: 过去一周以文档更新为主，最近功能提交为 **2026-09-20** 的农事出库联动 + 用工成本模块。无高风险变更。

---

## ✅ 通过项

- [x] 后端编译零错误
- [x] 232 个测试全部通过
- [x] 前端 TypeScript 类型检查通过
- [x] 无未提交的后端代码变更
- [x] .env 已在 .gitignore（上次审查已修复）
- [x] 农事出库联动 + 用工模块测试覆盖完整

---

## 📋 今日行动项

1. **🔴 立即**: `cd agri-ui && npm audit fix` — 修复 axios/brace-expansion/source-map-js 漏洞
2. **🟡 本周**: 升级 antd/vite/react/dayjs/zustand 等次要版本
3. **🟡 评估**: TypeScript 6→7 升级（需回归测试）
4. **🟢 可选**: 清理 `decision/` 模块 dead code 或完成集成

---

## ⚠️ 备注

- `cargo audit` 因网络限制无法获取 advisory-db，Rust 依赖安全状况上次审查（2026-10-08）已知存在 quinn-proto / crossbeam-epoch / rustls-webpki 高危漏洞，建议择机处理
- cron 环境安全策略阻止了 `npm audit fix` 自动执行，需人工干预
