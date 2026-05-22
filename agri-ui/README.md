# Agri-IoT 前端

智慧农业物联网监控系统前端，基于 React + TypeScript + Vite。

## 技术栈

- **框架**: React 19 + TypeScript 6
- **UI**: Ant Design 6 + CSS Modules
- **图表**: ECharts 6 + echarts-for-react
- **状态管理**: Zustand 5
- **路由**: React Router 7
- **构建**: Vite 8

## 快速开始

```bash
# 安装依赖
npm install

# 开发模式（端口 3001）
npm run dev

# 构建到后端静态目录
npm run build
```

构建输出自动写入 `agri-server/static/`，由后端 fallback 服务托管。

## 项目结构

```
src/
├── components/     # 公共组件
│   ├── Charts/         # ECharts 封装
│   ├── Layout/         # 布局（Header/Sidebar）
│   ├── ControlPanel/   # 设备控制面板
│   ├── AIAnalysisReport/   # AI 分析报告
│   ├── AISummaryPanel/     # AI 摘要面板
│   ├── AISystemStatus/     # AI 系统状态
│   ├── EmergencyBanner/    # 紧急情况横幅
│   ├── EmergencyRules/     # 紧急规则
│   ├── HealthScoreBar/     # 健康评分
│   ├── KnowledgeStats/     # 知识库统计
│   ├── MetricRow/          # 指标行
│   ├── OperationTimeline/  # 操作时间线
│   ├── TodoList/           # 待办事项
│   └── ZoneOverviewRow/    # 区域概览行
├── pages/          # 页面
│   ├── Dashboard/      # 仪表盘
│   ├── AIDecisions/    # AI 决策中枢
│   ├── ZoneDetail/     # 区域详情
│   ├── NodeList/       # 节点列表
│   ├── DataQuery/      # 数据查询
│   ├── RuleList/       # 规则列表
│   └── Settings/       # 系统设置
├── services/       # API 服务
│   ├── api.ts          # REST API 封装
│   ├── weather.ts      # 天气 API
│   └── sse.ts          # Server-Sent Events 实时数据
├── stores/         # Zustand 状态管理
│   ├── dashboardStore.ts
│   ├── realtimeStore.ts
│   └── zoneStore.ts
├── theme/          # 主题配置（Ant Design + ECharts）
├── types/          # TypeScript 类型定义
├── styles/         # 全局样式
├── App.tsx         # 根组件
└── main.tsx        # 入口
```

## 后端 API

所有 API 通过 `/api/v1/*` 访问，由 `agri-server`（Axum）提供。开发模式下 Vite 代理到 `localhost:3001`。
