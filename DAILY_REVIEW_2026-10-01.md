# Agri-Iot 每日代码审查报告
**日期**: 2026-10-01  
**检查范围**: 后端 Rust、前端 TypeScript、依赖安全、近期提交

---

## 1. 编译状态

| 组件 | 状态 | 说明 |
|------|------|------|
| `cargo build` | ✅ 通过 | 0 错误，48 警告 |
| `tsc --noEmit` | ✅ 通过 | 无类型错误 |
| ESLint | ✅ 通过 | 无 lint 问题 |
| 测试总计 | ✅ **228 通过** | agri-core 136 + agri-server 70 + agri-mqtt 22 |

---

## 2. 安全问题

### npm audit — ⚠️ 2 个高危漏洞

| 包 | 版本 | 来源 | 严重性 |
|----|------|------|--------|
| `axios` | 1.19.0 | 直接依赖 | 🔴 高危（10+ CVE） |
| `brace-expansion` | 5.0.9 | 间接（eslint→minimatch） | 🔴 高危（DoS） |

**axios 漏洞详情**：
- GHSA-vh66-26gq-q6x8：fetch adapter 原型污染
- GHSA-c29m-xwm3-cm6r：ReDoS（fromDataURI）
- GHSA-mghh-pgcx-3jjj：ReDoS（shouldBypassProxy）
- GHSA-3pq3-5fj3-cg6v：HTTP/2 adapter 绕过 DNS 代理
- 另有多个原型污染、Header Injection、SSRF 相关 CVE

**修复方案**：
```bash
cd agri-ui && npm install axios@latest  # 1.19.0 已是当前最新，需关注 axios 官方补丁
npm audit fix                           # brace-expansion 自动升级至安全版本
```

> ⚠️ axios 1.x 最新仍受部分 CVE 影响，建议评估迁移至 native fetch（项目已用 node 22+，可直接使用）。

### cargo audit — ⚠️ 网络限制无法执行

安全扫描因网络策略被阻止，建议手动运行：
```bash
cargo audit --db /tmp/advisory-db
```

---

## 3. Clippy 警告分析（48 个）

### P1：大量未使用代码（决策引擎子系统）

以下结构体/函数已构建但从未被调用，疑似废弃的 AI 决策系统残骸：

| 警告 | 文件 |
|------|------|
| `struct DecisionEngine` | `decision/` 目录 |
| `struct FlowContext` | `decision/` 目录 |
| `struct ApprovalGate` | `decision/notification/escalator.rs` |
| `struct NotificationMsg` | `notification/` 目录 |
| `struct ShiftRouter/ShiftSlot/RouteResult` | 调度相关 |
| `fn run_escalation / write_log / query_log / query_recent` | 日志/升级 |
| `struct LlmStage` 字段 `provider/retrieval` 未读 | `decision/stages/llm_stage.rs` |
| `enum RainIntensity` / `WindState` / `VentState` 变体未构造 | 天气枚举 |

**建议**：这些是 AI 决策重构后遗留的代码。若确认不再使用，建议删除或加 `#[allow(dead_code)]` 减少噪音。

### P2：类型设计问题

| 警告 | 位置 | 建议 |
|------|------|------|
| `Err` variant 过大 | `stock.rs` parse 函数 | 考虑拆分错误类型或 boxing |
| `AnalysisQuery.crop_id` 字段未读 | `yield.rs:248` | 确认是否需要，否则移除 |

---

## 4. 最近提交（近5日）

| Commit | 说明 |
|--------|------|
| `b1cd1b0` | 修复 anomaly.rs SQL 查询未 await（P0 已修复） |
| `f923b6d` | docs: AGENTS.md 记录农事出库联动 |
| `9ee0d03` | feat: 用工成本页面 + 农事总用量 + warnings 展示 |
| `53a131d` | feat: 015_labor.sql 用工表 + yield 净利含人工 |
| `41d7640` | test: labor.rs RED 阶段测试 |

最近提交集中在**用工成本模块**和**农事出库联动**，质量良好，测试先行。

---

## 5. 改进建议

| 优先级 | 建议 | 预估工作量 |
|--------|------|-----------|
| 🔴 P0 | 解决 axios 10+ 高危 CVE，评估迁移到 native fetch | 中 |
| 🟡 P1 | 清理 decision/ 目录废弃代码（约 500+ 行） | 小 |
| 🟡 P1 | 移除 `AnalysisQuery.crop_id` 未用字段 | 极小 |
| 🟢 P2 | `cargo audit` 离线扫描（已在本地有 advisory-db） | 极小 |
| 🟢 P2 | TypeScript 构建脚本名统一（当前无 `type-check` script） | 极小 |

---

## 6. 总结

- ✅ 编译、测试、TypeScript 零错误
- ✅ 今日已修复一个 P0 bug（SQL 未 await）
- ⚠️ **axios 10+ 高危漏洞**需尽快处理（后端 Rust 侧无已知安全漏洞）
- ⚠️ 决策引擎大量死代码，影响代码可维护性
- 依赖整体干净，无已知 Rust CVE
