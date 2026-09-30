# Agri-Iot 每日代码审查报告
**日期：2026-09-29（周二）**

---

## 一、编译状态

| 组件 | 状态 | 说明 |
|------|------|------|
| `cargo build -p agri-server` | ✅ 通过 | ~35 warnings，无 errors |
| `tsc --noEmit` | ✅ 通过 | 无 TypeScript 错误 |
| `cargo test --workspace` | ✅ 通过 | 228 tests（agri-core 136 + agri-server 70 + agri-mqtt 22） |

---

## 二、安全漏洞（cargo audit）

### 🔴 高危（需优先处理）

| 依赖 | 当前版本 | 修复版本 | CVE | 说明 |
|------|---------|---------|-----|------|
| `quinn-proto` | 0.11.14 | ≥0.11.15 | RUSTSEC-2026-0185 | 远程内存耗尽，通过无序流重组触发 |
| `rustls-webpki` | 0.101.7 / 0.102.8 | ≥0.103.12 | RUSTSEC-2026-0098/0099/0104 | 证书名称约束绕过 + CRL panic |

**排查方式**：`rustls-webpki` 通过 `tokio-rustls` → `reqwest` 间接依赖；`quinn-proto` 目前无 direct 依赖，可能为测试/条件编译引入。需 `cargo tree -i rustls-webpki` 确认传递路径。

### 🟡 中危（可安排跟进）

| 依赖 | 当前版本 | 说明 |
|------|---------|------|
| `rsa` | 0.9.10 | Marvin Attack 时序侧信道，无修复版本（CVE-2023-35948） |
| `crossbeam-epoch` | 0.9.18 | 指针解引用缺陷，升级至 ≥0.9.20 |
| `yaml-rust` | 0.4.5 | unmaintained，无活跃维护 |
| `rustls-pemfile` | 2.2.0 | unmaintained |
| `anyhow` | 1.0.102 | unsoundness（Error::downcast_mut） |
| `spin` | 0.9.8 | yanked |

### 🟢 npm audit
- **0 vulnerabilities** ✅

---

## 三、未提交变更概览

**33 个文件修改，+885 / -1409 行**

### 后端改动要点

| 文件 | 变更 | 评价 |
|------|------|------|
| `agri-mqtt/src/bin/broker.rs` + `broker.rs` | 重构 Config 构建：从 `Config::default()` + HashMap 改为 struct literal（`Config { ..Default::default() }`） | ✅ 代码更清晰，符合 Rust idiomatic |
| `agri-server/src/decision/notification/mod.rs` | `ChannelType::SMS` → `Sms` | ⚠️ 命名风格变更（与 `WeCom`/`DingTalk` 等不一致），建议统一 |
| `agri-server/src/rule_engine/anomaly.rs` | `fn median(vals: &mut Vec<f64>)` → `&mut [f64]` | ✅ 更通用，接收任何切片 |
| `agri-server/src/mixing.rs` | 移除 `let now = Utc::now().timestamp()`（未使用变量） | ✅ 清理 dead code |
| `agri-server/src/mqtt_ws.rs` | 简化版本比较逻辑 | ✅ 可读性提升 |
| `agri-server/src/ws_handler.rs` | `max(1).min(5000)` → `.clamp(1, 5000)` | ✅ 更简洁 |
| `agri-core/src/ai/embedding.rs` + `llm.rs` | 新增 `#[allow(dead_code)]` 抑制字段警告 | ⚠️ 应评估是否真的需要这些字段，或加注释说明用途 |
| `agri-server/src/ai_routes.rs` | 测试断言更新（品种数 >100） | ✅ |

### 前端改动要点

| 文件 | 变更 | 评价 |
|------|------|------|
| `ZoneDetail.tsx` | 改用 `Promise.all()` 并发加载 zone + nodes，加 cancelled flag 防内存泄漏 | ✅ 性能优化 + bug 修复 |
| `Dashboard.tsx`, `DataQuery.tsx` 等 | 多处 UI 交互改进 | 待人工 review |
| `FarmLog.module.css` | 新增 58 行样式 | 待人工 review |
| `package-lock.json` | 1371 行变更 | ⚠️ 可能包含不安全版本升级，需核查 |

### npm 可升级包（npm outdated）

| 包名 | Current | Latest |
|------|---------|--------|
| `react` | 19.2.6 | 19.3.0 |
| `react-dom` | 19.2.6 | 19.3.0 |
| `antd` | 6.3.7 | 6.6.5 |
| `vite` | 8.1.3 | 8.3.1 |
| `typescript` | 6.0.3 | 7.0.2 |
| `dayjs` | 1.11.20 | 1.11.23 |
| `axios` | 1.19.0 | 1.20.0 |
| `zustand` | 5.0.13 | 5.0.15 |

---

## 四、代码规范检查

| 项目 | 状态 | 说明 |
|------|------|------|
| 变量命名一致性 | ⚠️ | `ChannelType::SMS` → `Sms` 破坏与 `WeCom`/`DingTalk` 的命名一致性 |
| 死代码清理 | ⚠️ | `#[allow(dead_code)]` 用于抑制警告，但 `EmbedData.index`、`Usage.total_tokens` 等字段实际未用 |
| 错误处理 | ✅ | ZoneDetail 新增 cancelled flag 防止 async 内存泄漏 |
| 函数设计 | ✅ | `median()` 改用 slice 参数，提升通用性 |
| 并发模式 | ✅ | `Promise.all()` 替代串行 fetch，改善首屏加载 |

---

## 五、待办事项

### P0（立即处理）
1. **升级 `rustls-webpki`** — 多个高危 CVE，通过 `cargo tree -i rustls-webpki` 确认传递依赖路径后升级 `tokio-rustls`/`hyper-rustls`
2. **升级 `quinn-proto`** — 远程内存耗尽漏洞，检查是否有实际使用

### P1（本周内）
3. **统一 `ChannelType` 命名** — 将 `Sms` 改回 `SMS` 以匹配其他枚举值风格
4. **审核 `#[allow(dead_code)]` 字段** — 确认是否真的需要保留，或可删除
5. **升级 React 生态包** — react 19.2.6→19.3.0、antd 6.3.7→6.6.5、vite 8.1.3→8.3.1

### P2（可延后）
6. **排查 `rsa` Marvin Attack** — 如项目使用 RSA 加密，考虑替换为 `ring` 或 `ecdsa` 方案
7. **审核 package-lock.json 变更** — 确认是否引入新版本依赖

---

## 六、总结

**整体状态：🟢 健康**

- 编译、类型检查、测试全绿
- 有 28 个文件待提交，大部分为代码清理和性能优化
- 主要风险在 Rust 依赖安全漏洞（rustls-webpki、quinn-proto）
- 前端改动质量较高，ZoneDetail 并发加载和取消标志是好的实践

**建议优先处理 P0 安全漏洞，其余按 P1/P2 顺序跟进。**
