# 每日代码审查报告 — 2026-10-04

## 总体状态：GREEN（可部署）

---

## 编译与测试

| 检查项 | 结果 |
|--------|------|
| Rust 编译 | ✅ 通过（48 个警告，无错误） |
| TypeScript 类型检查 | ✅ 通过（0 错误） |
| 单元测试总数 | ✅ 228 个全过（core 136 + mqtt 22 + server 70） |

---

## 安全问题（需关注）

### 🔴 高危：axios 多个安全漏洞

```
npm audit fix 可修复
```

| 漏洞 | 描述 | 影响 |
|------|------|------|
| GHSA-vh66-26gq-q6x8 | 原型污染 → 篡改出站请求 | 高 |
| GHSA-9fr6-4gfg-395g | 原型污染 → 覆写 HTTP 方法 | 高 |
| GHSA-c29m-xwm3-cm6r | ReDoS 阻塞事件循环 | 高 |
| GHSA-mghh-pgcx-3jjj | ReDoS 在代理绕过逻辑 | 高 |
| GHSA-4hqw-qxg8-jxx2 | Header 注入 | 高 |
| GHSA-m8m8-qj5v-23w3 | Socket 劫持 | 高 |
| GHSA-r4gj-5m52-g5wh | maxRedirects:0 未生效，可绕过 SSRF 防护 | 高 |

**建议**：`cd agri-ui && npm audit fix` 升级 axios 到安全版本。

### 🟡 中危：brace-expansion DoS

```
GHSA-q2hr-2g5m-vwhr — 栈溢出风险
GHSA-6j4f-fj2g-mc7p — 递归深度失控
```

**建议**：`npm audit fix` 同步升级。

### 注：cargo audit 不可用

网络限制导致 RustSec advisory DB 无法拉取。建议定期手动检查关键依赖：

```bash
# 离线检查（如有条件）
git clone --depth 1 https://github.com/RustSec/advisory-db.git /tmp/advisory-db
cargo audit --db /tmp/advisory-db
```

---

## 代码质量

### 🟡 死代码累积（decision/notification 子系统）

以下代码从未被构造/调用，属于未完成的 AI 决策重构遗留：

| 文件 | 未使用项 |
|------|----------|
| `decision/notification/escalator.rs` | `EscalationChain`, `NotificationDispatch`, `Notifier` trait |
| `decision/stages/llm_stage.rs` | `FlowContext`, `StageOut`, `DecisionEngine` |
| `decision/stages/*` | 天气状态枚举、风状态枚举、决策日志结构 |
| `decision/notification/approval_gate.rs` | `ApprovalGate`, `ShiftRouter`, `ShiftSlot` |

**建议**：
- 若这些功能暂时不启用，建议加 `#[allow(dead_code)]` 或暂存分支清理
- 若计划近期上线，应尽快接入路由并移除 warning

### 🟢 最近提交回顾

| Commit | 内容 | 评估 |
|--------|------|------|
| `b1cd1b0` | 修复 anomaly.rs SQL 查询未 await + 每日审查 | ✅ 正确修复 |
| `f923b6d` | 文档更新 | ✅ |
| `9ee0d03` | 用工成本页面 + 农事记录总用量字段 | ✅ |
| `53a131d` | labor.sql + yield 净利含人工成本 | ✅ |
| `41d7640` | labor.rs RED 测试 | ✅ TDD 规范 |

---

## 改进建议

### P0（立即处理）
1. **升级 axios**：`cd agri-ui && npm audit fix`，修复 7 个高危安全漏洞

### P1（近期处理）
2. **清理死代码**：decision/notification 子系统（约 400 行）未启用，加 `#[allow(dead_code)]` 或归档

### P2（可选）
3. **cargo audit 自动化**：编写脚本缓存 RustSec DB，绕过网络限制定期检查

---

## 统计

- 后端测试：228 passed / 0 failed
- 前端类型错误：0
- 安全漏洞：7 high（axios）+ 2 high（brace-expansion）
- 未使用代码警告：48 条（主要为 decision 子系统）
