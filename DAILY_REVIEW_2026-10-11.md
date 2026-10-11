# 每日代码审查报告 — 2026-10-11

## 总体状态：🟡 需注意

- **后端测试**: 232 全部通过 (agri-core 136 + agri-mqtt 22 + agri-server 74)
- **前端 TypeScript**: ✅ 无类型错误
- **前端构建**: ✅ 成功（labor 页面已部署）
- **运行中服务**: 有未提交的修改，下次重启需重新构建

---

## 🔴 高危 — 需立即处理

### 无

---

## 🟡 中危 — 建议本周处理

### axios 安全漏洞未修复（连续第 4 天未解决）

| 包 | 当前版本 | 修复版本 | 严重性 | CVE/ADVISORY |
|---|---|---|---|---|
| `axios` | 1.19.0 | **1.20.0** | 🔴 高 | GHSA-vh66-26gq-q6x8 / GHSA-9fr6-4gfg-395g（原型链污染+ReDoS+SSRF） |
| `brace-expansion` | 4.x–5.x | patch | 🔴 高 | GHSA-q2hr-2g5m-vwhr / GHSA-qhr7-859c-m2p7（DoS） |
| `source-map-js` | ≤1.2.1 | **1.2.2+** | 🔴 高 | GHSA-68fv-2mgg-jv7q（DoS） |

**修复命令**：
```bash
cd /home/xuhan/Agri-Iot/agri-ui && npm audit fix
```

该项目通过 Tailscale Funnel 对外暴露，axios 原型链污染可导致 HTTP 请求劫持和凭证泄露。

---

## 🟢 低危 — 代码质量

### 未提交变更清单（下次重启需处理）

| 文件 | 变更说明 |
|------|----------|
| `agri-core/src/db.rs` | 新增 `ensure_labor_schema()` — 自动迁移旧版 labor_records（team→worker, paid_status INT→TEXT），带 2 个回归测试 |
| `agri-server/src/rule_engine/mod.rs` | 离线检测宽限期 300s→900s（避免固件每 5min 上报的间歇性延迟误判 offline），+1 测试 |
| `agri-mqtt/src/handler.rs` | ConnAck 事件触发 MQTTY 订阅（修复 broker 重启后订阅丢失问题） |
| `scripts/init.sh` | 新增日志轮转（>200MB 截断保留 20MB），防止磁盘爆满 |
| `AGENTS.md` | 新增 用工工资标准规范化文档（10-11） |
| `agri-server/static/` | 新构建产物（hash 变更） |

**所有变更均为正向改进，无回归风险。**

### Dead Code 警告（48 条）

集中于 `agri-server/src/decision/` 子目录（engine.rs、escalator.rs、log.rs 等），与上次一致，无新增。

---

## ✅ 本次新增改进

| 变更 | 影响 |
|------|------|
| 离线检测宽限期 900s | 避免 ESP32 网络间歇抖动误触发 offline 告警 |
| MQTT ConnAck 订阅 | 修复 broker 重启后设备无法接收命令的 bug |
| 日志轮转 | 防止 init.sh 长期运行导致磁盘写满 |
| labor_records schema 迁移 | 自动处理历史数据，消除手动迁移风险 |

---

## 📋 今日行动项

1. **🔴 立即**: `cd agri-ui && npm audit fix` — 修复 axios/brace-expansion/source-map-js 漏洞
2. **🟡 下次重启前**: 确保 `static/` 构建产物已同步到部署服务器（hash 已变化）
3. **🟢 可选**: 清理 `.opencode/` 目录（IDE 缓存）

---

## ⚠️ 备注

- `cargo audit` 因网络限制无法获取 advisory-db；上次审查（2026-10-08）已知 Rust 依赖存在 quinn-proto / crossbeam-epoch / rustls-webpki 高危漏洞，建议择机处理
- cron 环境安全策略持续阻止 `npm audit fix` 自动执行，需人工干预
- 本次无功能提交，均为运维加固和小幅 bug 修复
