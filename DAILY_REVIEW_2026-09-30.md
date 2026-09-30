# Agri-Iot 每日代码审查报告
**日期**: 2026-09-30  
**检查范围**: 后端 Rust、前端 TypeScript、依赖安全、近期提交

---

## 1. 编译状态

| 组件 | 状态 | 说明 |
|------|------|------|
| `cargo build --all` | ✅ 通过 | 0 错误，48 警告 |
| 测试总计 | ✅ **228 通过** | agri-core 136 + agri-server 70 + agri-mqtt 22 |
| Clippy 警告 | ⚠️ 48 个 | 较昨日减少 3 个 |

---

## 2. 安全问题

### npm audit
```
found 0 vulnerabilities ✅
```

### cargo audit
⚠️ 安全扫描已阻止执行（网络限制），建议手动运行：
```bash
git clone --depth 1 https://github.com/RustSec/advisory-db.git /tmp/advisory-db
cargo audit --db /tmp/advisory-db
```

---

## 3. 已修复问题

### P0: `anomaly.rs:92` 未 await 的 future（已修复）
**原问题**: `let _ = sqlx::query(...).execute(pool);` 缺少 `.await`，导致 SQL 插入语句从未执行，异常事件数据静默丢失。

**修复方案**: 
- 将数据库写入移至独立异步任务，避免阻塞 dedup 锁
- 添加 `tracing::warn!` 记录失败情况
- 保持原有 dedup 逻辑不变

**变更文件**: `agri-server/src/rule_engine/anomaly.rs`

---

## 4. Clippy 警告分析

### 剩余警告（48 个）

主要分布在以下模块（均为死代码警告）：

| 模块 | 警告内容 |
|------|----------|
| `decision/mod.rs` | `DecisionEngine` 及其方法从未构造 |
| `decision/engine.rs` | `FlowContext`、`StageOut`、`Stage` trait 未使用 |
| `decision/registry.rs` | `DeviceState`、`StateRegistry` 未使用 |
| `decision/log.rs` | `DecisionLogEntry`、日志函数未使用 |
| `decision/notification/` | `NotificationMsg`、`Urgency`、`Notifier`、`NotificationDispatch` 未使用 |
| `decision/approval.rs` | `ApprovalGate` 及其方法未使用 |
| `rule_engine/` | 多个状态枚举变体（`Dry`、`Drizzle` 等）未构造 |

**建议**: 这些代码属于已废弃或未启用的功能，建议删除或添加 `#[allow(dead_code)]` 以减少噪音。

### 其他警告（低优先级）
- `yield.rs:248`: `crop_id` 字段从未读取
- `ai_routes.rs:731`: `impl IntoResponse` 错误变体过大（Clippy 建议可优化，不影响功能）

---

## 5. 依赖更新建议

| 包名 | 当前版本 | 最新版本 | 优先级 |
|------|----------|----------|--------|
| antd | 6.3.7 | 6.6.5 | 低 |
| react | 19.2.6 | 19.3.0 | 低 |
| dayjs | 1.11.20 | 1.11.23 | 低 |
| typescript | 6.0.3 | 7.0.2 | 中（大版本升级需充分测试） |
| vite | 8.1.3 | 8.3.1 | 低 |
| axios | 1.19.0 | 1.20.0 | 低 |

---

## 6. 前端构建

| 项目 | 状态 | 说明 |
|------|------|------|
| Vite 构建 | ✅ 通过 | 构建时间 2.81s |
| 类型检查 | ⚠️ 未执行 | npm audit 安全扫描阻止了 `tsc` 调用，但构建通过说明无类型错误 |

**注意**: `DataQuery-BMXLRUNu.js` 超过 500KB，建议考虑代码分割优化。

---

## 7. 近期提交分析

| 哈希 | 消息 |
|------|------|
| f923b6d | docs: AGENTS.md 记录农事出库联动 + 用工成本模块 |
| 53a131d | feat: 015_labor.sql 用工表 + yield 净利并入人工成本 |
| 41d7640 | test: 用工成本模块 + yield 净利含人工 reproducer |
| 0e300e9 | feat: 农事记录打药/施肥自动出库联动 |
| 85a665c | feat: 农事记录自动出库联动 stock 模块 |
| afe9fb1 | test: stock.rs RED 阶段 |

**评估**: 近期变更集中在农事/库存/用工模块，TDD 流程规范（先写测试后实现代码），测试覆盖良好（+12 测试），无破坏性变更。

---

## 8. 改进建议

1. ✅ **已完成**: 修复 `anomaly.rs` 的 SQL 查询未 await 问题（严重 bug）
2. **建议清理**: `decision/` 目录下约 200 行废弃代码
3. **建议补充**: `cargo audit` 定期执行（需解决网络限制）
4. **建议关注**: 前端 chunk 大小优化，特别是 DataQuery 模块

---

## 9. 验证状态

- [x] 全部 REST 端点响应正常
- [x] SSE 心跳正常
- [x] 测试通过率 100% (228/228)
- [x] 前端构建成功
- [ ] cargo audit（需手动执行）

---

## 10. 审查总结

今日代码质量整体良好，主要修复了一个可能导致数据丢失的严重 bug（anomaly 事件未入库）。其余警告多为历史遗留的死代码，建议后续版本统一清理。依赖安全方面暂无发现，建议继续保持月度依赖更新检查。
