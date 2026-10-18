# 每日代码审查报告 — 2026-10-05

## 编译状态

| 组件 | 状态 | 说明 |
|------|------|------|
| agri-server | ✅ 通过 | 48 warnings，无 error |
| agri-core / agri-mqtt | ✅ 通过 | clippy 无错误 |
| agri-ui (tsc) | ✅ 通过 | 0 类型错误 |

## 近期提交（近 10 条）

```
e74e415 docs: 每日代码审查报告 2026-10-04
6144722 docs: 每日代码审查报告 2026-10-04
b1cd1b0 fix: 修复 anomaly.rs SQL 查询未 await 问题 + 每日代码审查报告
f923b6d docs: AGENTS.md 记录农事出库联动 + 用工成本模块
9ee0d03 feat(ui): 用工成本页面 + 农事记录总用量字段与出库 warnings 展示 + 重新构建前端
53a131d feat: 015_labor.sql 用工表 + yield 净利并入人工成本
41d7640 test: 用工成本模块 + yield 净利含人工 reproducer（labor.rs RED）
0e300e9 feat: 农事记录打药/施肥自动出库联动（创建/编辑/删除 + 集成测试）
85a665c feat: 农事记录自动出库联动 stock 模块（parse_amount/扣减/回滚）
afe9fb1 test: 农事记录自动出库联动 reproducer（stock.rs RED）
```

**评估**：功能交付节奏良好，TDD 流程规范（RE→GREEN→REFACTOR），最近重点在库存联动+用工成本模块。

---

## 安全漏洞

### 🔴 高危 — Node.js (axios)

| CVE | 问题 | 影响 |
|-----|------|------|
| GHSA-vh66-26gq-q6x8 | 原型污染劫持 HTTP 请求 | 高 |
| GHSA-9fr6-4gfg-395g | 默认实例原型污染覆盖 HTTP 方法 | 高 |
| GHSA-c29m-xwm3-cm6r | ReDoS 冻结 Node 事件循环 | 高 |
| GHSA-mghh-pgcx-3jjj | ReDoS O(N²) 代理 bypass | 高 |
| GHSA-r4gj-5m52-g5wh | maxRedirects:0 可绕过导致 SSRF | 高 |
| GHSA-m8m8-qj5v-23w3 | HTTP adapter 原型污染 → socket 劫持 | 高 |

**修复**：`npm audit fix` 或升级到 axios 1.7.0+（建议 1.7.9 最新稳定版）。

### 🔴 高危 — Rust (quinn-proto)

| ID | 问题 | 修复 |
|----|------|------|
| RUSTSEC-2026-0185 | 远程内存耗尽（超大 out-of-order 数据重组） | 升级 quinn-proto >= 0.11.15 |

### 🟡 中危 — Rust (rustls-webpki)

| ID | 问题 | 修复 |
|----|------|------|
| RUSTSEC-2026-0099 | Name constraints 通配符证书错误接受 | 升级 >= 0.103.12 |
| RUSTSEC-2026-0104 | CRL 解析可达 panic | 升级 >= 0.103.13 |
| RUSTSEC-2026-0049 | CRL Distribution Point 匹配逻辑错误 | 升级 >= 0.103.10 |

### 🟡 中危 — Rust (sqlx)

| ID | 问题 | 修复 |
|----|------|------|
| RUSTSEC-2024-0363 | 二进制协议 Misinterpretation（截断/溢出 Casts） | 升级 sqlx >= 0.8.1（**Breaking change**） |

### 🟢 低危 — 已标记警告（无需紧急处理）

| 组件 | 版本 | 状态 |
|------|------|------|
| crossbeam-epoch | 0.9.18 | 升级 >= 0.9.20 |
| rsa | 0.9.10 | Marvin 侧信道攻击，**无修复版本**，本项目未直接使用 RSA |
| paste | 1.0.15 | 不再维护 |
| rustls-pemfile | 2.2.0 | 不再维护 |
| yaml-rust | 0.4.5 | 不再维护 |
| anyhow | 1.0.102 | unsound downcast_mut，低影响 |
| spin | 0.9.8 | yanked |

---

## 代码风格

**clippy warnings（48 条，均为 unused 字段/函数，无逻辑错误）**：
- `agri-server/src/decision/notification/escalator.rs`：`run_escalation` 函数未调用
- `agri-server/src/decision/stages/llm_stage.rs`：`provider`、`retrieval` 字段声明但未读取
- `agri-server/src/yield.rs`：`AnalysisQuery.crop_id` 字段未使用

这些都是遗留死代码，不影响运行，可择机清理。

---

## 改进建议

| 优先级 | 建议 |
|--------|------|
| P0 | 升级 axios（`npm audit fix` 一键修复，高风险漏洞） |
| P0 | 升级 rustls-webpki（涉及 TLS 证书验证安全） |
| P1 | 升级 quinn-proto（高严重程度，内存耗尽风险） |
| P1 | 升级 sqlx 到 0.8.x（Breaking change，需评估迁移成本） |
| P2 | 清理 clippy unused 字段/函数（dead code） |
| P2 | 排查 rsa 依赖链（确认是否被 TLS 栈间接引入，如非必需可排除） |
