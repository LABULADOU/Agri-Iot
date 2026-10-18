# 每日代码审查报告 — 2026-10-16

## 一、测试状态

| 模块 | 通过 | 失败 |
|------|------|------|
| agri-core | 138 | 0 |
| agri-server | 76 | 0 |
| agri-mqtt | 22 | 0 |
| **总计** | **236** | **0** |

**全绿 ✓**

---

## 二、待提交变更（未 commit）

### 后端 Rust（核心修复）

| 文件 | 变更 | 说明 |
|------|------|------|
| `agri-core/src/models.rs` | Device/FarmOperation id 改为 String | 修复非 UUID 种子数据导致 500 |
| `agri-core/src/db.rs` | 新增 `ensure_labor_schema()` | 自动迁移旧版 labor_records（team→category/worker） |
| `agri-server/src/areas.rs` | 移除 Area struct 依赖，query_as → (String, ...) | 同上，防止 id 解析失败级联崩 |
| `agri-server/src/routes.rs` | 新增非 UUID area_id 容忍测试 | 回归测试 |
| `agri-server/src/rule_engine/dag.rs` | 空链返回 Ok(不报错) + 测试 | 修复 15 万+ warn 日志 |
| `agri-server/src/rule_engine/mod.rs` | 离线检测宽限期 300s→900s + 测试 | 避免新设备误判 offline |
| `agri-mqtt/src/handler.rs` | ConnAck 时自动重订阅 | 修复断线重连后 topic 丢失 |
| `agri-server/src/farm_log.rs` | 待确认 | — |
| `agri-server/src/main.rs` | 待确认 | — |

### 前端
- `agri-ui/src/pages/FarmLog/FarmLog.tsx` — 改动 +21/-?
- `agri-ui/src/pages/Labor/Labor.tsx` — 改动 +10/-?
- `agri-server/static/` — 需重新构建（当前有 D 删除 + 新 untracked .js）

---

## 三、编译警告（无错误）

### Clippy warning（48 条，均无错误）
- `decision/` 模块下多个 struct/method 未使用（框架代码，TODO 标记中）
- `yield.rs` 中 `AnalysisQuery.crop_id` 字段未读
- `rule_engine` 部分结构体未构造

### 已知 TODO（非阻塞）
| 文件 | 描述 |
|------|------|
| `decision/engine.rs` | 调度器未接入运行时 |
| `decision/mod.rs` | 三阶段管线骨架完成，未启动 |
| `decision/notification/*.rs` | 通知框架已定义，未实例化 |
| `dashboardStore.ts` | 推荐执行未实现 |

---

## 四、安全扫描

| 来源 | 结果 |
|------|------|
| `cargo audit` | ⚠️ 网络不可达，无法拉取 RustSec 数据库（需手动或网络恢复后执行） |
| `npm audit` | ⚠️ 3 个高危（axios 系列），均已提供 fix；brace-expansion / source-map-js 高危 |
| | 执行 `npm audit fix` 可解决 |

---

## 五、问题与建议

### P0 — 需立即处理
1. **前端产物不同步**：`static/` 下有大量删除的旧 bundle + 新建的 untracked 新 bundle，`index.html` 也已修改。需执行 `cd agri-ui && npm run build` 后统一提交，否则部署时静态资源混乱。

### P1 — 重要但非紧急
2. **未提交的代码变更较多**：约 10 个源文件 + AGENTS.md 待 commit，建议尽快提交或分批次提交。
3. **MQTT 重连订阅逻辑**：`handler.rs` 新增 ConnAck 重订阅是正确修复，但需确认 `sub_client` 参数在 `start_listener` 调用处已正确传入（检查 `main.rs` 或调用方）。

### P2 — 低优先级
4. **`cargo audit` 失败**：网络问题导致无法检查 Rust 依赖安全漏洞，下次网络恢复后需补充执行。
5. **决策模块 TODO**：`decision/` 目录大量未使用代码占用编译警告，可考虑暂 `#[allow(dead_code)]` 或清理。

---

## 六、最近提交记录

```
7eb63d0 docs: 每日代码审查报告 2026-10-14   ← 上次审查报告
2959bf0 docs: 每日代码审查报告 2026-10-13
436771e docs: 每日代码审查报告 2026-10-11
```

---

*审查时间：2026-10-16 | 审查范围：后端 Rust + 前端 TS + 安全扫描*
