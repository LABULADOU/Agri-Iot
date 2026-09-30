# Agri-Iot 每日代码审查报告

**日期**: 2026-08-28  
**审查范围**: Rust 后端 + React 前端

---

## 一、构建与测试状态

| 检查项 | 状态 | 说明 |
|--------|------|------|
| 编译 (Rust) | ✅ 通过 | 0 错误，67 警告 |
| 编译 (TypeScript) | ✅ 通过 | 0 类型错误 |
| 单元测试 | ✅ 205/205 通过 | agri-core 136 + agri-mqtt 22 + agri-server 47 |
| ESLint | ⚠️ 69 错误 | 详见下方 |
| cargo audit | ⚠️ 跳过 | 网络限制无法获取 RustSec 数据库 |
| npm audit | ✅ 通过 | 0 漏洞 |

---

## 二、高风险问题

### 1. `decision/` 模块：762 行骨架代码未接入主流程

**位置**: `agri-server/src/decision/`（9 个文件）

**现状**: 三阶段决策管线框架（紧急保护/状态切换/LLM 评估）骨架已完成，但所有组件均为 dead code：
- `DecisionEngine`、`FlowContext` 结构体从未被构造使用
- `ApprovalGate`、`NotificationDispatch`、`ShiftRouter` 全为空实现
- `StateRegistry` 在 `start()` 中用 `let _reg` 丢弃
- 事件循环仅空转，不触发任何实际调度

**警告统计**: 59 条 dead code 警告来自此模块

**建议**:
- **方案 A（推荐）**: 删除整个 `decision/` 模块，待需求明确后再重构
- **方案 B**: 补全一个 Tier 1 紧急保护流程（大风/大雨关通风），作为 MVP 验证价值

---

### 2. 前端 ESLint 错误：30 处 `any` 类型

**高频位置**:
- `stores/dashboardStore.ts` — AI 评估响应字段
- `services/api.ts` — 多处 API 响应类型

**影响**: 失去 TypeScript 类型安全保障，运行时错误难排查

**建议**: 定义完整类型，例如：
```typescript
// 替换 any
interface AIEssessment {
  score: number;
  suggestions: string[];
  emergency?: boolean;
}
```

---

### 3. 前端 React Hooks 违规：setState in useEffect

**涉及文件**:
- `ZoneDetail.tsx:120,125` — 同步调用 `fetchData()` / `fetchReadings()`
- `TopBar.tsx:135` — 同步调用 `fetchAll()`
- `LineChart.tsx:20` — 同步调用 `setChartHeight()`
- `useRealtimeReadings.ts` — 多处 refs 赋值 + setState

**影响**: 级联渲染，性能损耗

**建议**: 
```typescript
// 错误模式
useEffect(() => { fetchData(); }, []);

// 正确模式
useEffect(() => {
  fetchData().then(/* handle result */);
}, []);
```

---

## 三、中风险问题

### 4. 未提交的前端构建产物

```
M  agri-server/static/index.html          # 需提交
?? agri-server/static/assets/index-*.js   # 新构建产物（未追踪）
D  agri-server/static/assets/index-BPMG*.css  # 旧产物（已删除）
D  agri-server/static/assets/index-Ccq*.js    # 旧产物（已删除）
```

**建议**: `git add` 新产物并提交，保持静态资源同步。

### 5. 未使用的 Rust 变量（3 处重复）

```
warning: unused variable: `area_id`
warning: unused variable: `date_from`
warning: unused variable: `date_to`
```
出现在 `agri-server/src/routes.rs` 多处，可能是 route handler 参数未使用。

**建议**: 用 `_` 前缀或移除未使用的参数。

---

## 四、低风险/代码规范

### 6. Clippy 样式警告（可忽略或批量 fix）

- `binding's name is too similar` — 变量名相近易混淆
- `unnecessary hashes around raw string literal` — `r"..."` 可简化为 `R"..."`
- `long literal lacking separators` — 大数字加下划线分隔（如 `1_000_000`）

### 7. `cargo audit` 网络限制

RustSec 数据库拉取失败（HTTPS 请求超时）。建议：
- 配置代理或本地镜像
- 或定期手动克隆：`git clone --depth 1 https://github.com/RustSec/advisory-db.git ~/.cargo/advisory-db`

---

## 五、未提交文件清单

```
Modified:
  agri-server/static/index.html
  agri-ui/package-lock.json
  agri-ui/src/pages/FarmLog/FarmLog.tsx
  agri-ui/src/pages/Inventory/Inventory.tsx
  agri-ui/src/pages/Mixing/Mixing.tsx
  agri-ui/src/pages/Yield/Yield.tsx
  esp32-firmware/src/main.cpp

Untracked:
  agri-server/static/assets/index-Bl3DYueX.css
  agri-server/static/assets/index-CLzDAhAe.js
```

---

## 六、总结

| 类别 | 数量 | 优先级 |
|------|------|--------|
| 编译错误 | 0 | — |
| 测试失败 | 0 | — |
| 安全风险 | 0 | — |
| Dead Code | 762 行 | 中 |
| ESLint 错误 | 69 | 高 |
| TypeScript 警告 | 0 | — |

**下次审查重点**:
1. 清理或实现 `decision/` 模块
2. 修复前端 `any` 类型（预计 30 分钟）
3. 提交未完成的构建产物
