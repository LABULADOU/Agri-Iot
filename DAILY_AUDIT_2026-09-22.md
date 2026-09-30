# Agri-Iot 每日代码审查报告
**日期**: 2026-09-22  
**分支**: main  
**最近提交**: `f923b6d` docs: AGENTS.md 记录农事出库联动 + 用工成本模块

---

## 1. 编译状态

### Rust (`cargo check`)
- **结果**: ✅ 编译通过，无错误
- **警告**: 48 个 dead code 警告（主要为 decision/notification 子系统的未完成功能）

| 警告类型 | 数量 | 说明 |
|---------|------|------|
| 未使用函数 | 12 | decision/notification 模块（escalator, notifier 等） |
| 未使用字段 | 18 | 同一子系统 |
| 未使用结构体 | 10 | DecisionEngine, FlowContext, ApprovalGate 等 |
| 其他 | 8 | yield.rs 中 crop_id 字段未被读取 |

**建议**: decision/notification 子系统代码已写入但未接入主流程，考虑添加 `#[allow(dead_code)]` 或纳入构建。

### 前端 (`npm run build`)
- **结果**: ✅ 构建成功，耗时 2.80s
- **警告**: 部分 chunk 超过 500KB（DataQuery 1.1MB，api 535KB）——这是预期行为，前端路由已做 code splitting

### TypeScript/Lint (`npm run lint`)
- **结果**: ✅ 零错误

---

## 2. 测试覆盖

| 包 | 测试数 | 状态 |
|----|--------|------|
| agri-core | 136 | ✅ 全部通过 |
| agri-server | 70 | ✅ 全部通过 |
| agri-mqtt | 22 | ✅ 全部通过 |
| **总计** | **228** | ✅ |

### 新增测试（本次审查周期）
- `stock::tests` (9个): 农事出库联动测试
- `farm_log::tests` (4个): 创建/更新/删除联动测试
- `labor::tests` (6个): 用工记录 CRUD + 汇总测试
- `yield_rs::tests` (3个): 净利计算含人工成本测试

---

## 3. 安全审计

### Rust (`cargo audit`) — ⚠️ 发现 11 个漏洞

#### P0 高优先级（需立即处理）

| 漏洞 | 包 | 当前版本 | 建议版本 | 严重度 | 说明 |
|------|-----|---------|---------|--------|------|
| RUSTSEC-2026-0185 | quinn-proto | 0.11.14 | ≥0.11.15 | **7.5 High** | 远程内存耗尽，未排序流重组问题 |
| RUSTSEC-2026-0099 | rustls-webpki | 0.101.7 | ≥0.103.12 | 中 | 通配符名称约束接受错误 |
| RUSTSEC-2026-0104 | rustls-webpki | 0.101.7/0.102.8 | ≥0.103.13 | 中 | CRL 解析可达 panic |
| RUSTSEC-2026-0098 | rustls-webpki | 0.101.7/0.102.8 | ≥0.103.12 | 中 | URI 名称约束错误接受 |
| RUSTSEC-2026-0049 | rustls-webpki | 0.102.8 | ≥0.103.10 | 中 | CRL Distribution Point 匹配逻辑错误 |
| RUSTSEC-2024-0363 | sqlx | 0.7.4 | ≥0.8.1 | 中 | 二进制协议误解释（截断/溢出转换） |

#### P1 中优先级（可计划处理）

| 漏洞 | 包 | 当前版本 | 建议版本 | 严重度 |
|------|-----|---------|---------|--------|
| RUSTSEC-2026-0204 | crossbeam-epoch | 0.9.18 | ≥0.9.20 | 低 |
| RUSTSEC-2023-0071 | rsa | 0.9.10 | 无修复 | 5.9 中（Marvin 侧信道） |
| RUSTSEC-2026-0190 | anyhow | 1.0.102 | 待查 |  unsound |

#### 未维护警告（不影响安全，但需关注）

| 包 | 当前版本 | 状态 |
|------|---------|------|
| paste | 1.0.15 | 已停止维护 |
| rustls-pemfile | 1.0.4 / 2.2.0 | 已停止维护 |
| yaml-rust | 0.4.5 | 已停止维护 |
| spin | 0.9.8 | 已停止维护 |

### Node.js (`npm audit`)
- **结果**: ✅ 0 个漏洞

---

## 4. 最近变更分析

### 最近 5 次提交

| Commit | 主题 | 文件变更 |
|--------|------|---------|
| `f923b6d` | docs: AGENTS.md 记录农事出库联动 + 用工成本模块 | 文档更新 |
| `53a131d` | feat: 015_labor.sql 用工表 + yield 净利并入人工成本 | 迁移+后端 |
| `41d7640` | test: 用工成本模块 reproducer（labor.rs RED） | 测试 |
| `0e300e9` | feat: 农事记录打药/施肥自动出库联动 | 后端核心 |
| `85a665c` | feat: 农事记录自动出库联动 stock 模块 | 后端核心 |

### 变更评估

**✅ 正面**:
- 新功能有完整的测试覆盖（19个新增测试全部通过）
- 事务一致性：农事记录创建/更新/删除与库存联动正确
- 用工成本模块设计合理：amount = worker_count × work_hours × rate

**⚠️ 关注**:
- `decision/notification` 子系统代码量大（escalator, notifier, approval gate）但未接入主流程，产生 48 个 dead code 警告
- `AnalysisQuery.crop_id` 字段未被读取，可能是遗留代码

---

## 5. 改进建议

### 立即行动（P0）
1. **升级 quinn-proto** 到 0.11.15+（修复远程内存耗尽漏洞）
2. **升级 rustls-webpki** 到 0.103.x（修复多个证书验证漏洞）
3. 评估 sqlx 0.8 升级可行性（breaking change，需测试）

### 短期优化（P1）
4. 为 decision/notification 模块添加 `#[allow(dead_code)]` 或完成集成
5. 清理 `AnalysisQuery.crop_id` 未使用字段
6. 考虑替换未维护的依赖（yaml-rust → serde_yaml, spin → std::sync::Mutex）

### 长期规划
7. sqlx 0.7 → 0.8 升级需评估 breaking changes
8. rsa 库无安全修复，如不使用可考虑移除依赖

---

## 6. 总体评估

| 维度 | 状态 | 评分 |
|------|------|------|
| 编译质量 | ✅ 通过 | 8/10（dead code 警告可优化） |
| 测试覆盖 | ✅ 完整 | 10/10 |
| 前端质量 | ✅ 正常 | 9/10 |
| 安全性 | ⚠️ 需关注 | 6/10（11个Rust漏洞） |
| 代码整洁 | ⚠️ 一般 | 7/10（dead code 较多） |

**综合评级**: ⚠️ **需关注安全问题**

---

*报告生成时间: 2026-09-22T09:01 GMT+8*
