# 每日代码审查报告 — 2026-10-13

## 概览

| 项目 | 状态 |
|------|------|
| 后端编译 | ✅ 通过（48 warnings，无 error） |
| 测试 | ✅ 232/232 通过 |
| TypeScript | ✅ 无错误 |
| 安全漏洞 | ⚠️ 见下文 |
| 最近 commit | 上次审查至今无新 commit（未提交变更） |

---

## 一、后端 Rust

### 编译警告（48 条，均为 dead code）
警告均来自未完成/未启用的功能模块，不影响运行时：

| 类别 | 警告数 | 说明 |
|------|--------|------|
| `rule_engine/decision/` | ~35 | DecisionEngine、FlowContext、ApprovalGate、EscalationChain 等结构体/方法未使用 |
| `yield.rs` | 1 | `crop_id` 字段未读 |
| `labor.rs` / 其他 | ~12 | 若干辅助函数/字段未使用 |

**建议**：用 `#[allow(dead_code)]` 标记或移除尚未接入主流程的决策引擎代码，保持仓库整洁。

### 测试：232 通过，0 失败
- agri-core: 138
- agri-mqtt: 22
- agri-server: 74（含新增 labor schema 迁移测试）

---

## 二、前端 TypeScript

`tsc --noEmit` 输出为空，**无类型错误**。

---

## 三、安全漏洞

### Rust 依赖（cargo audit）
| 包 | 版本 | 问题 | 风险 |
|----|------|------|------|
| rustls-pemfile | 1.0.4 / 2.2.0 | unmaintained | 低（内部工具，非生产暴露） |
| yaml-rust | 0.4.5 | unmaintained | 低 |
| spin | 0.9.8 | yanked | 低 |
| **其他** | — | 共 15 条 | — |

### NPM 依赖（npm audit）
| 包 | 严重度 | 问题 |
|----|--------|------|
| axios | 高 | ReDoS + 多处 Prototype Pollution + Header Injection |
| brace-expansion | 高 | DoS（栈溢出） |
| source-map-js | 高 | DoS（indexed offset） |

**风险评估**：以上均为构建/开发时依赖，生产环境不直接暴露，风险可控。axios 是运行时依赖但仅用于内部管理系统，威胁面小。

---

## 四、未提交变更分析（6 个文件 + 静态资源重建）

### 1. `agri-core/src/db.rs` — labor_records 表迁移 ✅
**问题**：旧版 `labor_records` 使用 `team`/`paid_status INTEGER`，与当前代码（`worker`/`paid_status TEXT`）不兼容，而 `015_labor.sql` 的 `CREATE TABLE IF NOT EXISTS` 会跳过已存在的旧表。
**修复**：启动时检测 legacy 列，自动 DROP → RENAME → 新建 → 迁移数据 → 重建索引。
**风险**：低，迁移逻辑有完整测试覆盖。

### 2. `agri-server/src/areas.rs` — 设备 ID 解析修复 ✅
**问题**：`list_areas`/`get_area` 用 `sqlx::query_as::<_, Area>` 强类型反序列化，seed 数据中有 `'area-demo'` 这类非 UUID 文本 ID，触发 500 错误，前端 `Promise.all` 全部失败。
**修复**：改用 `(String, String, Option<String>, i64)` 元组，ID 作为普通文本解析。
**风险**：无，行为与前端预期一致。

### 3. `agri-mqtt/src/handler.rs` + `main.rs` — MQTT 订阅时机优化 ✅
**问题**：旧实现每 10s 轮询订阅，broker 重启后需等待下一个 10s 周期才能恢复；`ConnAck` 事件被忽略。
**修复**：在 `Event::ConnAck` 回调中一次性订阅所有 topic，去掉了定时轮询。
**风险**：低，行为更正确。

### 4. `agri-server/src/rule_engine/mod.rs` — 离线检测宽限期调整 ✅
**变更**：`300s → 900s`（3 倍固件采集间隔）。
**原因**：固件 READ_INTERVAL 为 300s，300s 宽限期仅允许丢 1 次数据即误判离线。
**新增测试**：`test_offline_detection_grace_period_900s`。
**风险**：低，离线检测延迟增加但误报率显著下降。

### 5. `scripts/init.sh` — 日志轮转 ✅
**新增**：`rotate_log_if_big()`，日志超 200MB 时截断保留末尾 20MB，子进程以 O_APPEND 打开 inode，无需重启。
**风险**：无。

### 6. `agri-ui/` 静态资源重建
32 个 asset 文件已更新（与源码同步），`index.html` 引用已更新。

---

## 五、建议行动项

| 优先级 | 事项 | 文件 |
|--------|------|------|
| P1 | 提交当前 6 处变更 + 静态资源重建 | — |
| P2 | 清理 decision engine dead code（~35 warnings） | `rule_engine/decision/` |
| P3 | axios 更新至安全版本（可选，当前风险低） | `agri-ui/package.json` |

---

## 六、无异常发现

- 无编译错误
- 无测试失败
- 无 TypeScript 类型错误
- 未提交变更均为修复性质，无回归风险
