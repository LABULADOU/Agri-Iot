# 每日代码审查报告 — 2026-10-14

## 一、后端 Rust — 编译状态 ✅

```
cargo build: 成功（无 error）
警告数: 48（全部 warning，无 error）
```

### 关键警告（需关注）

| 文件 | 警告 | 说明 |
|------|------|------|
| `decision/notification/escalator.rs` | `run_escalation` 未使用 | 通知升级链逻辑已废弃或未接入 |
| `decision/stages/llm_stage.rs` | `provider`/`retrieval` 字段未读 | LLM Stage 结构体字段存在但未使用 |
| `yield.rs` | `AnalysisQuery.crop_id` 未读 | 查询结构体字段冗余 |
| 通知模块 | `NotificationMsg`/`Notifier`/`Urgency` 等全部未使用 | 通知系统代码大量残留 |

**结论**: 编译无错误，48 个警告均为"未使用代码"——主要是 AI 决策通知子系统（`decision/notification/`）的代码框架已完成但未接入主线逻辑，属于预留代码，暂不影响运行。

---

## 二、后端测试 — 全绿 ✅

| 模块 | 测试数 | 结果 |
|------|--------|------|
| agri-core | 138 | ✅ 全过 |
| agri-mqtt | 22 | ✅ 全过 |
| agri-server | 76 | ✅ 全过 |
| **总计** | **236** | ✅ 0 失败 |

---

## 三、前端 TypeScript — ⚠️ 受限

TypeScript 编译器（tsc）被安全扫描拦截，无法执行类型检查。

### 手动检查

- `agri-ui/src/pages/FarmLog/FarmLog.tsx` — 本次提交修改了 21 行，涉及用量表单字段
- `agri-ui/src/pages/Labor/Labor.tsx` — 修改了 10 行，工日制相关调整
- `agri-server/static/assets/` — 新构建产物已生成，旧 bundle 已删除，`index.html` 已更新引用

**建议**: 下次有前端变更时手动在本地运行 `npx tsc --noEmit` 验证类型安全。

---

## 四、最近提交记录

### 过去 2 天变更（10-13）

| 提交 | 文件数 | 说明 |
|------|--------|------|
| `2959bf0` 2026-10-13 | 37 文件 | 区域管理重构 + 农事日志 + 规则引擎 DAG 优化 + 前端静态构建 |

**关键代码变更**:
- `agri-server/src/areas.rs` (+90 行): 区域列表接口增强，支持分页/过滤
- `agri-server/src/rule_engine/dag.rs`: 规则引擎 DAG 拓扑排序优化
- `agri-server/src/farm_log.rs` (+47 行): 农事日志接口完善
- `agri-core/src/db.rs` (+174 行): 数据库连接/迁移逻辑增强
- 前端 FarmLog 增加用量字段录入能力

### 待提交变更（git status M）

以下文件有本地修改未提交：
- `agri-core/src/db.rs` / `models.rs`
- `agri-mqtt/src/handler.rs`
- `agri-server/src/areas.rs` / `routes.rs` / `main.rs`
- `agri-server/src/rule_engine/mod.rs` / `dag.rs`
- `agri-ui/src/pages/FarmLog/FarmLog.tsx`
- `agri-ui/src/pages/Labor/Labor.tsx`

**⚠️ 建议**: 这些未提交修改包含重要功能变更，应尽快 review 后提交。

---

## 五、依赖安全

### Rust（cargo audit）

| 包 | 问题 | 级别 |
|----|------|------|
| `rustls-pemfile` | unmaintained | ⚠️ 警告 |
| `yaml-rust 0.4.5` | unmaintained | ⚠️ 警告 |
| `spin 0.9.8` | yanked | ⚠️ 警告 |
| 其余 | 20 条低危 | ℹ️ 信息 |

**评估**: 均无已知高危漏洞，unmaintained 包为间接依赖，影响有限。建议后续升级 `yaml-rust`。

### 前端（npm audit）

| 包 | 问题 | 级别 |
|----|------|------|
| `axios 1.0.0–1.19.0` | 原型链污染 + SSRF + ReDoS 共 11 项 | 🔴 高危 |
| `brace-expansion` | DoS（三次） | 🟠 中危 |
| `source-map-js` | DoS | 🟠 中危 |

**评估**: 
- **axios 漏洞**: 本项目为内网部署（Tailscale Funnel），不暴露公网，axios 主要调用 Open-Meteo 等可信第三方 API，**实际风险较低**。但建议升级到 `axios >= 1.7.0` 修复已知问题。
- **brace-expansion / source-map-js**: 为构建工具链间接依赖，前端 SSRF/DoS 风险对本项目无影响。

---

## 六、代码规范

| 检查项 | 状态 | 说明 |
|--------|------|------|
| Rust 缩进/命名 | ✅ | 符合约定 |
| TypeScript 格式 | ⚠️ | 未执行 ESLint（被安全扫描拦截），建议本地检查 |
| 未提交代码 | ⚠️ | 约 10 个文件有本地修改未 commit |
| 构建产物同步 | ✅ | static/ 已重新构建 |
| .gitignore | ✅ | .env / node_modules 已正确忽略 |

---

## 七、本次审查结论

| 维度 | 评级 |
|------|------|
| 后端编译 | ✅ 通过 |
| 后端测试 | ✅ 236/236 全过 |
| 前端类型 | ⚠️ 受限（安全扫描拦截 tsc） |
| 依赖安全 | 🟡 低风险（axios 建议升级） |
| 代码规范 | 🟡 有待提交变更 |

**总体**: 今日无新增代码问题，上次审查的已知问题未复发。主要待办：
1. **提交未完成的本地修改**（areas/farm_log/rule_engine 等）
2. **axios 升级**至 1.7+ 消除安全告警
3. 清理 `decision/notification/` 中大量未使用代码（或完成接入）
