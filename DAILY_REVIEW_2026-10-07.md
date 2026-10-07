# 每日代码审查报告 — 2026-10-07

## 一、后端编译状态

```
cargo check: ✓ 成功（无错误）
cargo test --workspace: 228 passed, 0 failed
  - agri-core: 136 tests ✓
  - agri-server: 70 tests ✓
  - agri-mqtt: 22 tests ✓
```

**警告统计：27 个**，全为 dead code（未使用的 struct/field/method）

| 优先级 | 问题 | 位置 | 建议 |
|--------|------|------|------|
| P2 | 决策模块为骨架代码，未接入调度 | `decision/mod.rs` 中 `_t1/_t2/_t3` 丢弃 flow | 加 `#[allow(dead_code)]` 或完成实现 |
| P3 | `DecisionEngine`/`FlowContext` 等类型未使用 | `decision/engine.rs`, `decision/mod.rs` | 同上 |
| P3 | 多个 `id` field 未被读取 | 多处 nodes.rs | 加 `#[allow(dead_code)]` 或清理 |
| P3 | 多个 variant 未构造 | emergency.rs 中 Dry/Detecting/Confirmed 等 | 加 allow 或启用 |

---

## 二、前端状态

```
npm run build: ✓ 成功（2.9s）
静态产物: agri-server/static/ 已更新
```

**Chunk 大小警告（需关注）：**

| Chunk | 大小 | gzip | 建议 |
|-------|------|------|------|
| DataQuery | 1,156 kB | 384 kB | 超大，需懒加载 |
| index (主 bundle) | 562 kB | 234 kB | 偏大 |
| api | 535 kB | 177 kB | 含所有 API 服务 |

**建议：**
- DataQuery 页面改为懒加载：`const DataQuery = lazy(() => import('./pages/DataQuery'))`
- 考虑将 ECharts 按需引入而非全局打包
- 检查是否有未使用的组件持续打包进主 bundle

---

## 三、安全审计（cargo audit）

### 🔴 高危（需优先修复）

| Crate | 版本 | 问题 | 严重性 | 修复方案 |
|-------|------|------|--------|----------|
| `sqlx` | 0.7.4 | Binary Protocol Misinterpretation (RUSTSEC-2024-0363) | 高 | 升级至 0.8.1 |

**评估：** sqlx 0.7 → 0.8 是 breaking change，需测试验证。当前 SQLite 使用场景风险较低。

### ⚠️ 中危

| Crate | 版本 | 问题 | 修复方案 |
|-------|------|------|----------|
| `rustls-webpki` | 0.101.7/0.102.8 | 证书解析漏洞 (3项) | 升级至 >=0.103.12 |
| `quinn-proto` | 0.11.14 | 内存耗尽 DoS | 升级至 >=0.11.15 |
| `crossbeam-epoch` | 0.9.18 | 指针解引用 | 升级至 >=0.9.20 |
| `rsa` | 0.9.10 | Marvin 侧信道攻击 | 无修复，项目未直接使用 RSA |

### ⚠️ 低危（unmaintained/yanked）

| Crate | 状态 | 说明 |
|-------|------|------|
| `rustls-pemfile` | 1.0.4 / 2.2.0 | unmaintained，传递依赖 |
| `yaml-rust` | 0.4.5 | unmaintained，传递依赖 |
| `spin` | 0.9.8 | yanked，传递依赖 |
| `paste` | 1.0.15 | unmaintained，传递依赖 |

**评估：** 低危项均为传递依赖，当前项目无直接安全影响。

---

## 四、Node.js 依赖

```bash
npm audit 未运行（权限限制）
axios 当前版本: 1.16.0（已知有漏洞）
```

**建议：** 升级 axios 至 1.20.0+ 修复已知漏洞

---

## 五、近期变更（最近 5 次 commit）

```
d961bda docs: 每日代码审查报告 2026-10-06
e74e415 docs: 每日代码审查报告 2026-10-04
6144722 docs: 每日代码审查报告 2026-10-04
b1cd1b0 fix: 修复 anomaly.rs SQL 查询未 await 问题
f923b6d docs: AGENTS.md 记录农事出库联动 + 用工成本模块
```

**观察：**
- 连续多日无功能性 commit，以文档和审查报告为主
- 农事出库联动和用工成本模块已上线（2026-09-20）
- 前端近期有较大重构（44 files changed）

---

## 六、改进建议

### P0（立即处理）
- 升级 `sqlx` 至 0.8.1（需回归测试）
- 升级 `rustls-webpki` 至 >=0.103.12

### P1（本周处理）
- 决策模块 dead code 清理（加 `#[allow(dead_code)]` 或完成实现）
- DataQuery 页面懒加载，减少主 bundle 体积
- 升级 `axios` 至 1.20.0+

### P2（本月处理）
- 前端 chunk 分析，识别未使用组件
- 迁移 unmaintained 传递依赖（rustls-pemfile, yaml-rust）

---

## 七、健康指标

| 指标 | 值 | 状态 |
|------|-----|------|
| 测试通过率 | 228/228 (100%) | ✅ |
| 编译错误 | 0 | ✅ |
| 编译警告 | 27 | ⚠️ |
| 高危漏洞 | 1 (sqlx) | ⚠️ |
| 前端构建 | 成功 | ✅ |
| Bundle 大小 | 1.1 MB (DataQuery) | ⚠️ 需优化 |

---

*报告生成时间：2026-10-07*
