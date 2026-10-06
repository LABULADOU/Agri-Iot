# 每日代码审查报告 — 2026-10-06

## 一、后端编译状态

```
cargo build: ✓ 成功（无错误）
cargo test --all: 228 passed, 0 failed
  - agri-core: 136 tests
  - agri-server: 70 tests  
  - agri-mqtt: 22 tests
```

**警告统计：48 个**，主要为 dead code（未使用的 struct/field/method）

| 优先级 | 问题 | 位置 | 建议 |
|--------|------|------|------|
| P2 | 决策模块为骨架代码，未接入调度 | `decision/mod.rs` 中 `let _t1/_t2/_t3` 丢弃 flow | 加 `#[allow(dead_code)]` 或完成实现 |
| P3 | `DecisionEngine`/`FlowContext` 等类型未使用 | `decision/engine.rs`, `decision/mod.rs` | 同上 |
| P3 | 多个 `id` field 未被读取 | 多处 | 加 `#[allow(dead_code)]` 或清理 |

---

## 二、前端状态

```
npm run build: ✓ 成功（4.10s）
tsc --noEmit: ✓ 无类型错误
```

**Chunk 大小警告：**
- `DataQuery-BMXLRUNu.js`: 1,156 kB（gzip: 384 kB）— 超大，建议代码分割
- `index-DKtpEnIc.js`: 562 kB — 主 bundle 偏大

**建议：**
- DataQuery 页面懒加载：`const DataQuery = lazy(() => import('./pages/DataQuery'))`
- 考虑将 ECharts 按需引入而非全局打包

---

## 三、安全审计

### Rust 依赖（cargo audit）

| 严重性 | Crate | 版本 | 问题 |
|--------|-------|------|------|
| ⚠️ | rustls-pemfile | 2.2.0 | unmaintained (RUSTSEC-2025-0134) |
| ⚠️ | yaml-rust | 0.4.5 | unmaintained (RUSTSEC-2024-0320) |
| ⚠️ | anyhow | 1.0.102 | unsound: Error::downcast_mut() (RUSTSEC-2026-0190) |
| ⚠️ | spin | 0.9.8 | yanked |

**评估：** 均为传递依赖，当前项目无直接安全影响。anyhow 的 unsoundness 在 IoT 场景影响有限。

### Node.js 依赖（npm audit）

| 严重性 | Package | 版本 | 问题 |
|--------|---------|------|------|
| 🔴 HIGH | axios | 1.19.0 | 12 项漏洞（范围含当前版本）|

**修复：** 升级至 1.20.0+ 已修复全部漏洞
```bash
cd agri-ui && npm install axios@^1.20.0
```

### Rust 依赖

| 严重性 | Crate | 变更 |
|--------|-------|------|
| ⚠️ | anyhow | 1.0.102 → 1.0.104（已执行 cargo update）|
| ⚠️ | rustls-pemfile | unmaintained（传递依赖，无直接影响）|
| ⚠️ | yaml-rust | unmaintained（传递依赖）|

---

## 四、最近提交记录（过去 3 天）

```
e74e415 docs: 每日代码审查报告 2026-10-04
6144722 docs: 每日代码审查报告 2026-10-04
b1cd1b0 fix: 修复 anomaly.rs SQL 查询未 await 问题 + 每日代码审查报告
f923b6d docs: AGENTS.md 记录农事出库联动 + 用工成本模块
9ee0d03 feat(ui): 用工成本页面 + 农事记录总用量字段与出库 warnings 展示
```

**变更要点：**
- 农事记录自动出库联动功能上线（Part1 + Part2）
- 用工成本模块完整实现
- yield 净利计算已包含人工成本
- anomaly.rs SQL 查询 bug 已修复

---

## 五、代码规范检查

| 检查项 | 状态 | 说明 |
|--------|------|------|
| 命名规范 | ✓ | 符合 Rust/TS 惯例 |
| 错误处理 | ✓ | internal_err() 统一封装 |
| 测试覆盖 | ✓ | 新增模块均有集成测试 |
| SQL 注入防护 | ✓ | 全部参数化查询 |
| 路径穿越防护 | ✓ | safe_path() 已实现 |
| 速率限制 | ✓ | RateLimiter 60 req/s |

---

## 六、改进建议

### 立即处理（今日）
1. **npm audit fix** — 修复 axios 高危漏洞
2. **清理决策模块 dead code** — 删除未使用的 DecisionEngine/FlowContext

### 本周处理
3. **axios 升级** — npm install axios@^1.20.0 修复 12 项高危漏洞
4. **决策模块 dead code** — DecisionEngine/FlowContext 未被使用，确认是否废弃
5. **DataQuery 懒加载** — 减少首屏 bundle 大小（1156KB chunk）

### 长期优化
6. 评估 axios 替代方案（如 native fetch API）
7. 考虑将决策引擎重构为更轻量的规则匹配

---

## 七、健康评分

| 维度 | 评分 | 说明 |
|------|------|------|
| 编译状态 | ✅ 9/10 | 警告偏多但不影响功能 |
| 测试覆盖 | ✅ 10/10 | 228 测试全过 |
| 安全性 | ⚠️ 7/10 | axios 需升级；anyhow 已更新；其他为传递依赖 |
| 代码质量 | ⚠️ 7/10 | dead code 较多 |
| 文档完整 | ✅ 9/10 | AGENTS.md 持续更新 |

**综合评分：8.2/10**

---

*报告生成时间：2026-10-06*
*下次审查：2026-10-07*
