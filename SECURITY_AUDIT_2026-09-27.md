# Agri-Iot 安全审查报告
**日期：** 2026-09-27
**扫描范围：** Cargo.lock (409 crates) / package-lock.json / Dockerfile / 固件

---

## 执行摘要

| 类别 | 漏洞数 | 警告数 | 状态 |
|------|--------|--------|------|
| Rust 依赖 (cargo audit) | 11 | 6 | 需处理 |
| 前端依赖 (npm audit) | 0 | - | 干净 |
| Docker 镜像 | N/A | - | 无相关配置 |
| ESP32 固件 | - | - | 需人工审查 |

---

## 一、Rust 依赖漏洞（按严重程度排序）

### P0 - 高危漏洞，建议尽快修复

#### 1. rustls-webpki 0.101.7 x 3 个漏洞
**影响路径：** agri-mqtt -> rumqttd v0.18.0 -> rustls v0.21.12 -> rustls-webpki 0.101.7

| ID | 问题 | CVSS | 说明 |
|----|------|------|------|
| RUSTSEC-2026-0099 | Name constraints 接受通配符证书 | 2.2 (Low) | 需 misissuance 才能利用 |
| RUSTSEC-2026-0104 | CRL 解析可达 panic (DoS) | 7.5 (High) | 默认禁用 CRL 验证，实际风险低 |
| RUSTSEC-2026-0098 | URI name 约束错误接受 | 2.2 (Low) | 本项目未使用 URI 约束 |

**修复方案：** 升级 rumqttd 至 0.20.0+（使用 rustls-webpki 0.102.1）或等待上游修复 0.103.x
**当前版本：** rumqttd = "0.18"

#### 2. rustls-webpki 0.102.8 x 3 个漏洞
**影响路径：** agri-mqtt -> rumqttc v0.24.0 -> rustls v0.22.4 -> rustls-webpki 0.102.8

| ID | 问题 | CVSS | 说明 |
|----|------|------|------|
| RUSTSEC-2026-0099 | Name constraints 接受通配符证书 | 2.2 (Low) | 同上 |
| RUSTSEC-2026-0104 | CRL 解析可达 panic (DoS) | 7.5 (High) | 默认禁用 CRL，实际风险低 |
| RUSTSEC-2026-0098 | URI name 约束错误接受 | 2.2 (Low) | 同上 |

**修复方案：** 升级 rumqttc 至 0.25.1+（注意：仍使用 rustls-webpki 0.102.8，未完全修复）
**当前版本：** rumqttc = "0.24"

#### 3. sqlx 0.7.4 - RUSTSEC-2024-0363
**问题：** Binary Protocol Misinterpretation，长度前缀溢出可导致协议混淆
**CVSS：** 未单独标注（DEF CON 已演示 exploit）
**影响评估：**
- 本项目使用 SQLite，非 PostgreSQL
- 漏洞主要针对 PostgreSQL 二进制协议层
- SQLite 后端不受此特定问题影响
- 但为安全起见仍建议升级
**修复方案：** 升级 sqlx 至 >= 0.8.1（当前最新稳定版 0.8.6，0.9.0 已发布）
**注意：** 需测试与现有代码的兼容性

### P1 - 中危漏洞，建议处理

#### 4. quinn-proto 0.11.14 - RUSTSEC-2026-0185
**问题：** 远程内存耗尽（CVSS 7.5 High）
**影响评估：**
- 直接依赖：reqwest v0.12.28 -> quinn 0.11.9 -> quinn-proto 0.11.14
- 潜在影响：若启用 HTTP/3 (h3) 特性，可通过 QUIC 连接触发
- 当前状态：项目未显式启用 http3 特性，风险较低但存在
**修复方案：** 升级 reqwest 至最新，或禁用 http3 特性
**当前版本：** reqwest = { version = "0.12", features = ["json", "rustls-tls", "gzip"] }

#### 5. crossbeam-epoch 0.9.18 - RUSTSEC-2026-0204
**问题：** fmt::Pointer 无效指针解引用
**影响路径：** rumqttd -> metrics-exporter-prometheus -> crossbeam-epoch
**修复方案：** 升级 rumqttd 至 0.20.0+ 时会自动解决（或使用 crossbeam-epoch >= 0.9.20）

### P2 - 低风险/信息类

#### 6. rsa 0.9.10 - RUSTSEC-2023-0071 (Marvin Attack)
**CVSS：** 5.9 (Medium)
**问题：** 时序侧信道攻击，可能恢复私钥
**影响评估：**
- 本项目使用 SQLite，rsa 为间接依赖（可能通过 sqlx-mysql）
- 攻击需要网络访问 + 精确计时能力，实施难度高
- 当前风险等级：低
**修复方案：** 暂无修复版本，等待 rsa 10.x 发布
**缓解措施：** 确保 rsa 不在关键加密路径上

#### 7. 废弃/不安全依赖（警告）

| 包名 | 版本 | 问题 | 状态 |
|------|------|------|------|
| paste | 1.0.15 | 停止维护 | 追踪中 |
| rustls-pemfile | 1.0.4 / 2.2.0 | 停止维护 | 追踪中 |
| yaml-rust | 0.4.5 | 停止维护 | 追踪中 |
| anyhow | 1.0.102 | unsound (Error::downcast_mut) | INFO 级，需关注 |
| spin | 0.9.8 | yanked | 需替换 |

---

## 二、前端依赖审查

### npm audit

0 vulnerabilities

### npm outdated（20 个过期包）

| 包名 | 当前 -> 最新 | 建议 |
|------|-------------|------|
| antd | 6.3.7 -> 6.6.5 | 建议升级（含安全修复） |
| react / react-dom | 19.2.6 -> 19.3.0 | 建议升级 |
| vite | 8.1.3 -> 8.3.1 | 建议升级 |
| dayjs | 1.11.20 -> 1.11.23 | 建议升级 |
| axios | 1.19.0 -> 1.20.0 | 建议升级 |
| eslint | 10.3.0 -> 10.11.0 | 建议升级 |
| typescript | 6.0.3 -> 7.0.2 | 大版本跳跃，需全面测试 |

---

## 三、Docker / 容器镜像

项目未配置 Dockerfile 或 docker-compose
- 跳过镜像安全审查
- 建议后续考虑容器化部署时使用 Alpine/scratch 基础镜像

---

## 四、ESP32 固件

依赖：PubSubClient, WebSockets, ArduinoJson, DHT Sensor Library
建议：定期检查 PlatformIO 核心库更新，关注 ArduinoJson 版本（已更新至 7.x）

---

## 五、修复优先级建议

| 优先级 | 行动 | 预计工作量 | 风险等级 |
|--------|------|-----------|----------|
| P0 | 升级 rumqttd 至 0.20.0+（解决 rustls-webpki 问题） | 中 | 高 |
| P0 | 升级 sqlx 至 0.8.x（需兼容性测试） | 高 | 高 |
| P1 | 升级 reqwest（解决 quinn-proto 问题） | 低 | 中 |
| P1 | 前端 antd/react/vite 等包升级 | 中 | 中 |
| P2 | 监控 rsa/anyhow/spin 后续修复版本 | 低 | 低 |

---

## 六、检测命令参考

bash
# Rust 安全扫描
cargo audit

# 前端安全扫描
cd agri-ui && npm audit

# 查看过期依赖
cd agri-ui && npm outdated

# 查看依赖树（定位漏洞来源）
cargo tree -i rustls-webpki
cargo tree -i sqlx
cargo tree -e features -i quinn-proto

---

报告生成：Hermes Agent (cron job)
下次扫描建议：2026-10-04（一周后）
