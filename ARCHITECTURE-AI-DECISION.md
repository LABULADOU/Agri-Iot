# Agri-iot AI 决策系统重构方案

> 版本: v1.0  
> 日期: 2026-05-18  
> 状态: 待实施

---

## 一、项目背景与愿景

### 1.1 当前状态
- 项目已完成基础架构：Rust 后端 + React 前端 + ESP32 固件
- 传感器已接入：DHT22（温湿度）
- 连通性测试通过，项目处于开发阶段

### 1.2 重构目标

```
传感器数据 + 气象数据 + 作物知识库
         ↓
    AI 决策中枢
         ↓
    环境调控 + 紧急保护
         ↓
    知识积累（越用越专业）
```

**核心价值：**
- 🌱 保障植物生长在舒适环境
- 🛡️ 减少病虫害发生
- ⚡ 紧急情况自动保护
- 📈 知识库持续学习进化

---

## 二、大棚物理模型

### 2.1 大棚结构

```
┌─────────────────────────────────────────────────────────────────┐
│                        钢结构连栋大棚                             │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  顶部通风口 (天窗) ←───── 卷膜器控制 ─────→ 开合度 0-100%  │    │
│  │                                                         │    │
│  │     ┌───────────────────────────────────────────────┐   │    │
│  │     │                                               │   │    │
│  │     │              种植区域                          │   │    │
│  │     │                                               │   │    │
│  │     │   [土壤传感器]  [土壤传感器]  [土壤传感器]     │   │    │
│  │     │                                               │   │    │
│  │     └───────────────────────────────────────────────┘   │    │
│  │                                                         │    │
│  │  侧面通风口 (侧窗) ←───── 卷膜器控制 ─────→ 开合度 0-100%  │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 通风系统特性

| 通风类型 | 控制方式 | 量程 | 说明 |
|----------|----------|------|------|
| 顶部通风 | 卷膜器 | 0-100% | 主要散热除湿，天窗式 |
| 侧面通风 | 卷膜器 | 0-100% | 辅助通风，侧窗式 |

**重要：卷膜器量程需要在项目落地时学习记录**

### 2.3 传感器配置

| 传感器类型 | 监测参数 | 用途 |
|------------|----------|------|
| 土壤温湿度传感器 | 土壤温度、土壤湿度 | 环境调控依据 |
| EC 值传感器 | 电导率 | 施肥策略判断（人工干预为主） |

---

## 三、知识库体系（核心）

### 3.0 Obsidian 知识库引擎

**选用 Obsidian 作为知识库管理工具**

- **优势**: 双向链接网络化知识、Markdown 格式、CLI 操作、模板系统、标签管理
- **存储**: Obsidian Vault 目录
- **数据流向**: Rust 后端 → 文件系统 → Obsidian Vault → AI 检索
- **同步机制**: 后端通过文件系统读写 Obsidian 笔记，Obsidian 作为人机交互界面

```rust
// agri-core/src/ai/knowledge.rs

use std::path::PathBuf;
use std::fs;

pub struct ObsidianKnowledge {
    vault_path: PathBuf,
}

impl ObsidianKnowledge {
    /// 初始化知识库连接
    pub fn new(vault_path: &str) -> Self {
        Self {
            vault_path: PathBuf::from(vault_path),
        }
    }
    
    /// 读取笔记内容
    pub fn read_note(&self, note_path: &str) -> Result<String> {
        let full_path = self.vault_path.join(note_path);
        fs::read_to_string(full_path)
            .map_err(|e| KnowledgeError::ReadFailed(e.to_string()))
    }
    
    /// 搜索知识（通过文件内容）
    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        // 使用 Rust 文件搜索或调用 obsidian CLI
        let output = Command::new("obsidian")
            .args(["search", "--query", query])
            .output()?;
        
        // 解析搜索结果
        ...
    }
    
    /// 创建/更新笔记（追加调控案例）
    pub fn append_case(&self, area_id: &str, case: &ControlCase) -> Result<()> {
        let path = format!("{}/cases/{}/{}.md", 
            self.vault_path.display(), 
            area_id,
            case.id
        );
        
        let content = self.render_case_note(case);
        fs::write(path, content)?;
        
        Ok(())
    }
}
```

**Obsidian Vault 结构:**
```
Agri-iot Knowledge/
├── 00-Crops/              # 作物知识库
│   ├── 番茄.md
│   ├── 黄瓜.md
│   └── ...
├── 01-Pests/              # 病虫害库
│   ├── 灰霉病.md
│   ├── 白粉病.md
│   └── ...
├── 02-Cases/              # 调控案例库（按区域组织）
│   ├── zone-1/
│   │   ├── 2026-05-18-高温调控.md
│   │   └── 2026-05-19-大风保护.md
│   └── zone-2/
├── 03-Weather/            # 气象规则
│   ├── 大风保护规则.md
│   ├── 暴雨保护规则.md
│   └── 降雪保护规则.md
├── 04-Templates/          # 模板
│   ├── crop-profile.md
│   ├── pest-knowledge.md
│   └── control-case.md
└── 05-Daily/             # 每日评估日记
    ├── 2026-05-18.md
    └── 2026-05-19.md
```

### 3.1 四层知识库架构

```
┌─────────────────────────────────────────────────────────────┐
│                     知识库层 (Knowledge Base)                 │
├───────────────┬───────────────┬───────────────┬─────────────┤
│   作物知识库   │   病虫害库    │   调控案例库   │  气象影响库  │
├───────────────┼───────────────┼───────────────┼─────────────┤
│ 最适区间参数   │  发生条件     │   决策记录     │ 气象-作物映射│
│ 生长阶段参数   │  预警信号     │   效果评估     │ 季节性规律   │
│ 环境阈值      │  防治方案     │   成功率统计   │ 极端天气应对 │
│ 通风偏好      │  紧急处理预案  │   知识置信度   │ 夜间特殊规则  │
└───────────────┴───────────────┴───────────────┴─────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                   向量数据库层 (Embedding)                    │
│        所有知识 → 向量化 → 语义检索（RAG 架构）              │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 知识库详细设计

#### 3.2.1 作物知识库 (crop_profiles)

```sql
CREATE TABLE crop_profiles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    variety TEXT,                    -- 品种
    growth_stages TEXT,              -- JSON: 生长阶段配置
    
    -- 环境参数区间
    soil_temp_min REAL,
    soil_temp_max REAL,
    soil_temp_optimal REAL,
    
    soil_moisture_min REAL,
    soil_moisture_max REAL,
    soil_moisture_optimal REAL,
    
    air_temp_min REAL,
    air_temp_max REAL,
    air_temp_optimal REAL,
    
    air_humidity_min REAL,
    air_humidity_max REAL,
    air_humidity_optimal REAL,
    
    ec_min REAL,                     -- EC 值区间
    ec_max REAL,
    ec_optimal REAL,
    
    -- 通风偏好
    ventilation_preference TEXT,     -- 'high'/'medium'/'low'
    wind_sensitivity REAL,           -- 对风的敏感度 0-1
    
    -- 向量库关联
    embedding_id TEXT,
    
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

#### 3.2.2 病虫害知识库 (pest_knowledge)

```sql
CREATE TABLE pest_knowledge (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    crop_types TEXT,                 -- 影响的作物类型（JSON数组）
    
    -- 触发条件
    trigger_conditions TEXT,          -- JSON:
    -- {
    --   "soil_temp_min": 20,
    --   "soil_temp_max": 30,
    --   "soil_moisture_min": 60,
    --   "humidity_min": 80,
    --   "duration_hours": 6
    -- }
    
    symptoms TEXT,                   -- 症状描述
    severity TEXT,                   -- 'low'/'medium'/'high'/'critical'
    
    -- 预防和治疗
    prevention TEXT,
    treatment TEXT,
    medication TEXT,                 -- 推荐药剂
    
    -- 紧急程度
    is_emergency BOOLEAN DEFAULT FALSE,  -- 是否需要紧急处理
    emergency_action TEXT,          -- 紧急情况下的自动处理
    
    -- 知识来源和置信度
    source TEXT,                     -- 来源：专家/文献/经验
    confidence REAL DEFAULT 0.8,    -- 置信度 0-1
    
    embedding_id TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

#### 3.2.3 调控案例库 (control_cases)

```sql
CREATE TABLE control_cases (
    id TEXT PRIMARY KEY,
    area_id TEXT,
    crop_profile_id TEXT,
    
    -- 情境描述
    situation TEXT,                  -- JSON: 环境状态快照
    -- {
    --   "soil_temp": 25,
    --   "soil_moisture": 45,
    --   "air_temp": 30,
    --   "air_humidity": 85,
    --   "ec_value": 2.0,
    --   "weather_condition": "sunny"
    -- }
    
    weather_forecast TEXT,            -- 未来天气预报
    
    -- 采取的行动
    action_taken TEXT,               -- JSON:
    -- {
    --   "top_vent": {"target": 80, "duration_min": 30},
    --   "side_vent": {"target": 50, "duration_min": 60},
    --   "irrigation": {"duration_min": 15}
    -- }
    
    manual_override BOOLEAN DEFAULT FALSE,  -- 是否人工干预
    
    -- 结果评估
    outcome TEXT,                    -- 'success'/'partial'/'failed'
    effect_rating INTEGER,          -- 1-5 分
    health_improvement REAL,        -- 健康分提升幅度
    
    -- 耗时
    action_duration_minutes INTEGER,
    recovery_time_minutes INTEGER,  -- 恢复到舒适状态的时间
    
    notes TEXT,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    embedding_id TEXT
);

-- 案例有效性追踪
CREATE TABLE case_effectiveness (
    case_id TEXT,
    assessment_time DATETIME,
    soil_temp_score REAL,            -- 当时各项评分
    soil_moisture_score REAL,
    pest_occurred BOOLEAN,
    notes TEXT
);
```

#### 3.2.4 气象知识库 (weather_knowledge)

```sql
CREATE TABLE weather_knowledge (
    id TEXT PRIMARY KEY,
    condition_type TEXT,             -- 'wind'/'rain'/'snow'/'storm'/'heat'/'frost'
    
    -- 影响参数
    thresholds TEXT,                 -- JSON: 触发阈值
    -- 大风: {"speed_kmh": 40}      -- 风速 > 40km/h
    -- 大雨: {"precipitation_mm": 10, "duration_hours": 2}
    -- 下雪: {"temperature_celsius": 2, "snow_probability": 0.7}
    
    -- 保护规则
    protection_rules TEXT,           -- JSON:
    -- {
    --   "action": "close_top_vent",
    --   "requires_confirmation": false,  -- 紧急情况跳过确认
    --   "priority": "critical"
    -- }
    
    -- 时间规则
    time_constraints TEXT,           -- JSON:
    -- {
    --   "night_only": true,        -- 夜间限制
    --   "nocturnal_warning": true  -- 需要夜间提醒
    -- }
    
    contact_required BOOLEAN DEFAULT FALSE,  -- 是否需要联系管理人员
    contact_urgency TEXT,           -- 'normal'/'urgent'/'critical'
    contact_message TEXT,            -- 预定义消息模板
    
    notes TEXT,
    embedding_id TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

---

## 四、数据模型扩展

### 4.1 新增数据库表

```sql
-- 大棚设备配置表（卷膜器量程等）
CREATE TABLE greenhouse_config (
    id TEXT PRIMARY KEY,
    area_id TEXT NOT NULL,
    
    -- 顶部通风配置
    top_vent_min_percent REAL DEFAULT 0,
    top_vent_max_percent REAL DEFAULT 100,
    top_vent_current_percent REAL DEFAULT 0,
    top_vent_id TEXT,                -- 设备ID
    
    -- 侧面通风配置
    side_vent_min_percent REAL DEFAULT 0,
    side_vent_max_percent REAL DEFAULT 100,
    side_vent_current_percent REAL DEFAULT 0,
    side_vent_id TEXT,
    
    -- 其他设备
    irrigation_device_id TEXT,
    fertigation_device_id TEXT,
    
    -- 紧急联系人
    emergency_contact_name TEXT,
    emergency_contact_phone TEXT,
    
    -- 量程学习状态
    top_vent_calibrated BOOLEAN DEFAULT FALSE,
    side_vent_calibrated BOOLEAN DEFAULT FALSE,
    calibration_date DATETIME,
    
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 传感器配置表
CREATE TABLE sensor_config (
    id TEXT PRIMARY KEY,
    area_id TEXT NOT NULL,
    sensor_type TEXT,                -- 'soil_temp'/'soil_moisture'/'ec'/'air_temp'/'air_humidity'
    device_id TEXT,
    calibration_offset REAL DEFAULT 0,  -- 校准偏移
    is_active BOOLEAN DEFAULT TRUE,
    last_reading DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 气象数据表
CREATE TABLE weather_data (
    id TEXT PRIMARY KEY,
    area_id TEXT,
    source TEXT,                     -- 'api'/'local'/'forecast'
    
    -- 当前位置
    temperature REAL,
    humidity REAL,
    wind_speed REAL,                 -- km/h
    wind_direction TEXT,
    precipitation REAL,              -- mm
    snow_probability REAL,           -- 0-1
    uv_index REAL,
    
    -- 预测数据
    forecast_hour INTEGER,          -- 预测时间偏移（小时）
    
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 环境评估记录
CREATE TABLE env_assessments (
    id TEXT PRIMARY KEY,
    area_id TEXT,
    crop_profile_id TEXT,
    
    -- 评估时间
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    -- 评分 (0-100)
    overall_score REAL,
    soil_temp_score REAL,
    soil_moisture_score REAL,
    ec_score REAL,
    air_temp_score REAL,
    air_humidity_score REAL,
    
    -- 偏差分析
    deviations TEXT,                 -- JSON: {参数: {current, optimal, deviation}}
    
    -- 病虫害风险
    pest_risks TEXT,                 -- JSON: [{pest_id, risk_level, reason}]
    
    -- 建议
    recommendations TEXT,            -- JSON: [{action, priority, reason}]
    
    -- 气象预测影响
    weather_impact TEXT,             -- JSON: 未来几小时的预测影响
    
    -- 紧急情况标记
    is_emergency BOOLEAN DEFAULT FALSE,
    emergency_type TEXT
);

-- 知识库更新日志
CREATE TABLE kb_update_log (
    id TEXT PRIMARY KEY,
    update_type TEXT,                -- 'case_added'/'knowledge_curated'/'feedback_received'
    source TEXT,                     -- 'manual'/'auto'/'ai_review'
    content_summary TEXT,
    effectiveness_score REAL,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### 4.2 更新现有表

```sql
-- devices 表增加新字段
ALTER TABLE devices ADD COLUMN area_id TEXT;
ALTER TABLE devices ADD COLUMN capabilities TEXT;  -- 已有，保留

-- readings 表扩展
ALTER TABLE readings ADD COLUMN soil_temp REAL;
ALTER TABLE readings ADD COLUMN soil_moisture REAL;
ALTER TABLE readings ADD COLUMN ec_value REAL;
```

---

## 五、AI 决策引擎

### 5.1 决策流程

```
┌─────────────────────────────────────────────────────────────────┐
│                        AI 决策中枢                               │
│                                                                 │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │   数据融合    │ →  │   紧急检测   │ →  │  环境评估    │      │
│  │  Sensor+Weather│    │  Weather Alert│   │  评分系统    │      │
│  └──────────────┘    └──────────────┘    └──────────────┘      │
│         │                   │                   │               │
│         ↓                   ↓                   ↓               │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    决策路由                              │    │
│  │                                                          │    │
│  │  IF 紧急情况 ──────→ 立即执行（跳过确认）                 │    │
│  │  ELSE IF 建议调控 ─→ 生成方案 → 等待确认                 │    │
│  │  ELSE → 无操作                                          │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              │                                    │
│                              ↓                                    │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │   执行反馈   │ →  │   案例记录   │ →  │  知识更新    │      │
│  │  效果评估    │    │  归档存储    │    │  持续学习    │      │
│  └──────────────┘    └──────────────┘    └──────────────┘      │
└─────────────────────────────────────────────────────────────────┘
```

### 5.2 决策规则优先级

```
优先级 1 (最高): 紧急保护
  - 大风/暴雨 → 立即关闭顶部通风
  - 下雪 → 紧急通知 + 暂停自动操作

优先级 2: 极端环境调控
  - 温度严重超出范围 → 强制通风/遮阳
  - 湿度严重超标 → 强制除湿

优先级 3: 常规优化
  - 舒适度轻微下降 → 建议调控
  - 病虫害风险上升 → 预防建议

优先级 4: 被动监测
  - 传感器数据采集
  - 知识库更新
```

### 5.3 环境评估算法

```rust
// agri-core/src/ai/assess.rs

pub struct EnvironmentAssessment {
    pub overall_score: f64,           // 0-100
    pub soil_temp_score: f64,
    pub soil_moisture_score: f64,
    pub ec_score: f64,
    pub air_temp_score: f64,
    pub air_humidity_score: f64,
    
    pub deviations: Vec<Deviation>,   // 偏离详情
    pub trend: String,                // 'improving'/'stable'/'declining'
    pub weather_impact: WeatherImpact,
}

pub fn calculate_parameter_score(
    current: f64,
    optimal: f64,
    min: f64,
    max: f64
) -> f64 {
    // 基础分 100
    if current >= min && current <= max {
        let deviation = ((current - optimal) / ((max - min) / 2.0)).abs();
        return 100.0 * (1.0 - deviation * 0.5).max(0.0);
    }
    
    // 超出范围，指数衰减
    let deviation = if current < min {
        (min - current) / min
    } else {
        (current - max) / max
    };
    return (100.0 * (-deviation * 2.0).exp()).max(0.0)
}
```

---

## 六、紧急情况处理逻辑

### 6.1 紧急情况定义

```rust
// agri-core/src/ai/emergency.rs

#[derive(Debug, Clone, PartialEq)]
pub enum EmergencyType {
    StrongWind,      // 大风
    HeavyRain,       // 大雨
    Snow,            // 降雪
    ExtremeHeat,     // 极端高温
    ExtremeCold,     // 极端低温
    SystemFailure,   // 系统故障
}

#[derive(Debug, Clone)]
pub struct EmergencyRule {
    pub emergency_type: EmergencyType,
    
    // 触发条件
    pub condition: TriggerCondition,
    
    // 响应动作
    pub immediate_action: Action,          // 立即执行
    pub requires_confirmation: bool,       // 是否需要确认
    
    // 通知规则
    pub contact_required: bool,
    pub contact_urgency: Urgency,
    pub notification_template: String,
    
    // 时间约束
    pub night_alert: bool,                 // 夜间是否需要额外提醒
}

pub struct TriggerCondition {
    pub weather_param: WeatherParam,
    pub operator: CompareOp,
    pub threshold: f64,
    pub duration_minutes: Option<u32>,     // 持续时间要求
}

// 响应动作
pub struct Action {
    pub action_type: ActionType,
    pub target_device: Option<String>,
    pub target_value: Option<f64>,          // 如通风口开合度
}
```

### 6.2 紧急情况详细规则

#### 规则 1: 大风保护 ⚠️ CRITICAL

```yaml
trigger:
  type: strong_wind
  condition:
    wind_speed_kmh: "> 40"
    duration_minutes: 0  # 立即触发
  
action:
  immediate: true
  action_type: CLOSE_DEVICE
  target: top_vent
  target_value: 0  # 完全关闭
  
requires_confirmation: false  # 紧急情况，跳过确认

contact:
  required: true
  urgency: normal
  message: "检测到大风(>{wind_speed}km/h)，已自动关闭顶部通风口"
  night_alert: true
```

**逻辑说明：**
- 风速超过 40km/h 立即触发
- 立即关闭顶部通风（防止风灌入造成结构损坏）
- 不需要人工确认（系统自动执行）
- 记录操作日志并发送通知

#### 规则 2: 大雨保护 ⚠️ CRITICAL

```yaml
trigger:
  type: heavy_rain
  condition:
    precipitation_mm_per_hour: "> 10"
    duration_minutes: 0
  
action:
  immediate: true
  action_type: CLOSE_DEVICE
  target: top_vent
  target_value: 0  # 完全关闭
  
requires_confirmation: false

contact:
  required: true
  urgency: normal
  message: "检测到大雨({precipitation}mm/h)，已自动关闭顶部通风口"
  night_alert: true
```

**逻辑说明：**
- 降雨量超过 10mm/h 立即触发
- 防止雨水灌入大棚
- 夜间同样适用

#### 规则 3: 降雪保护 🚨 CRITICAL + 人工介入

```yaml
trigger:
  type: snow
  condition:
    temperature_celsius: "< 3"
    snow_probability: "> 0.6"
    # 或
    precipitation_type: "snow"
  
action:
  immediate: true
  action_type: CLOSE_TOP_VENT
  
requires_confirmation: false
pauses_auto_mode: true  # 暂停自动模式，等待人工处理

contact:
  required: true
  urgency: CRITICAL  # 最高优先级
  message: |
    ⚠️ 气象预报显示降雪风险！
    当前温度: {temp}°C
    降雪概率: {probability}%
    
    已自动关闭顶部通风口。
    请立即前往现场检查大棚状况！
    
  night_alert: true
  night_additional_contact: true  # 夜间还需额外通知
```

**逻辑说明：**
- 降雪风险时关闭通风口
- **暂停自动模式**，不进行其他自动调控
- 立即发送最高级别警报
- 夜间无人值守时：
  - 多次重复通知
  - 如果有备用联系人，依次通知
  - 持续监控雪量积累

#### 规则 4: 极端高温

```yaml
trigger:
  type: extreme_heat
  condition:
    temperature_celsius: "> 38"
    duration_minutes: 10
  
action:
  immediate: false
  requires_confirmation: true  # 高温需要确认
  actions:
    - target: top_vent
      value: 100  # 最大限度通风
    - target: side_vent
      value: 100
    - target:遮阳网  # 如果有
      action: deploy

contact:
  required: true  # 通知管理人员观察
  urgency: normal
  message: "极端高温警告，温度已达{temp}°C，正在执行紧急通风"
  night_alert: true
```

#### 规则 5: 极端低温

```yaml
trigger:
  type: extreme_cold
  condition:
    temperature_celsius: "< 5"
    duration_minutes: 15
  
action:
  immediate: false
  requires_confirmation: true
  actions:
    - target: top_vent
      value: 0  # 关闭通风
    - target: 加热设备  # 如果有
      action: start

contact:
  required: true
  urgency: normal
```

### 6.3 夜间特殊处理

```rust
// agri-core/src/ai/night_mode.rs

pub struct NightModeConfig {
    pub enabled: bool,
    pub start_time: NaiveTime,     // e.g., 18:00
    pub end_time: NaiveTime,        // e.g., 06:00
    
    // 夜间额外规则
    pub enhanced_monitoring: bool,
    pub reduced_action_threshold: f64,  // 夜间更保守的阈值
    
    // 紧急情况联系人轮换
    pub night_contact_list: Vec<Contact>,
}

impl NightModeConfig {
    pub fn is_night_time(&self, now: DateTime<Utc>) -> bool {
        let local = now.with_timezone(&Tz::LOCALE);
        let current_time = local.time();
        
        if self.start_time > self.end_time {
            // 跨午夜
            current_time >= self.start_time || current_time <= self.end_time
        } else {
            current_time >= self.start_time && current_time <= self.end_time
        }
    }
}
```

**夜间处理增强：**
1. 降雪/暴雨通知间隔缩短（每15分钟重复）
2. 紧急联系人列表轮换通知
3. 增加短信/电话等多渠道通知
4. 记录夜间事件详情供白天复盘

---

## 七、设备控制逻辑

### 7.1 卷膜器控制

```rust
// agri-core/src/ai/ventilation.rs

pub struct VentilationController {
    // 配置
    pub top_vent_range: (f64, f64),  // 量程 0-100
    pub side_vent_range: (f64, f64),
    
    // 当前位置（通过传感器或状态反馈获取）
    pub top_vent_current: f64,
    pub side_vent_current: f64,
}

impl VentilationController {
    /// 计算达到目标所需的开合度
    /// 考虑当前状态、量程限制、变化速率
    pub fn calculate_target_position(
        &self,
        target_temp: f64,
        current_temp: f64,
        target_humidity: f64,
        current_humidity: f64,
        ventilation_type: VentType
    ) -> VentilationDecision {
        let range = match ventilation_type {
            VentType::Top => self.top_vent_range,
            VentType::Side => self.side_vent_range,
        };
        
        // 温控逻辑：温度越高，通风越大
        let temp_score = ((target_temp - current_temp) / 10.0).clamp(-1.0, 1.0);
        
        // 湿控逻辑：湿度越高，通风越大
        let hum_score = ((current_humidity - target_humidity) / 20.0).clamp(0.0, 1.0);
        
        // 综合决策
        let open_percent = ((temp_score + hum_score) / 2.0 * 100.0)
            .clamp(range.0, range.1);
        
        VentilationDecision {
            target_percent: open_percent,
            estimated_duration_minutes: self.estimate_duration(open_percent),
            priority: if temp_score > 0.5 || hum_score > 0.7 {
                ActionPriority::High
            } else {
                ActionPriority::Normal
            }
        }
    }
    
    /// 紧急关闭（用于大风/暴雨/下雪）
    pub fn emergency_close(&mut self, target: VentType) -> Action {
        match target {
            VentType::Top => {
                self.top_vent_current = self.top_vent_range.0;
            }
            VentType::Side => {
                self.side_vent_current = self.side_vent_range.0;
            }
        }
        
        Action {
            command: "CLOSE".to_string(),
            device_type: "vent".to_string(),
            target_percent: 0,
            requires_confirmation: false,  // 紧急情况
            is_emergency: true,
            notification: Some("紧急关闭通风口".to_string())
        }
    }
}
```

### 7.2 量程学习流程

```rust
// agri-core/src/ai/calibration.rs

/// 首次部署时学习卷膜器量程
pub async fn calibrate_ventilator(
    device_id: &str,
    area_id: &str
) -> Result<CalibrationResult> {
    // 1. 记录当前状态
    let initial_state = read_device_state(device_id)?;
    
    // 2. 发送全开命令
    send_command(device_id, Command::SetVent(100))?;
    sleep(Duration::from_secs(30)).await;  // 等待执行
    
    // 3. 获取实际开合度（通过位置传感器或电流反馈）
    let full_open_reading = read_device_state(device_id)?;
    
    // 4. 发送全关命令
    send_command(device_id, Command::SetVent(0))?;
    sleep(Duration::from_secs(30)).await;
    
    let full_close_reading = read_device_state(device_id)?;
    
    // 5. 计算量程
    let range = (full_close_reading.position, full_open_reading.position);
    
    // 6. 保存配置
    save_calibration(area_id, device_id, range)?;
    
    Ok(CalibrationResult {
        device_id: device_id.to_string(),
        range,
        calibration_date: Utc::now(),
        verified: true
    })
}
```

---

## 八、EC 值与施肥策略

### 8.1 EC 值监测逻辑

```rust
// agri-core/src/ai/fertigation.rs

pub struct ECManager {
    // EC 值区间配置
    pub optimal_ec_min: f64,
    pub optimal_ec_max: f64,
    pub warning_threshold_low: f64,
    pub warning_threshold_high: f64,
    
    // 施肥建议规则
    pub fertigation_rules: Vec<FertigationRule>,
}

#[derive(Debug)]
pub enum ECRecommendation {
    NoAction,           // EC 正常
    IncreaseEC {       // 需要增加肥料
        suggested_delta: f64,
        reason: String
    },
    DecreaseEC {       // 需要降低肥料
        suggested_delta: f64,
        reason: String
    },
    ManualIntervention {  // 需要人工介入
        reason: String,
        urgency: Urgency
    }
}

impl ECManager {
    /// 分析 EC 值并给出建议
    /// 注意：施肥策略人工干预程度较大，此处主要用于监测和提醒
    pub fn analyze_ec(
        &self,
        current_ec: f64,
        trend: &ECT rends,
        area_id: &str
    ) -> ECRecommendation {
        match current_ec {
            x if x < self.warning_threshold_low => {
                ECRecommendation::ManualIntervention {
                    reason: format!(
                        "EC值({:.2})严重偏低，可能需要补充肥料",
                        current_ec
                    ),
                    urgency: Urgency::High
                }
            }
            x if x < self.optimal_ec_min => {
                ECRecommendation::IncreaseEC {
                    suggested_delta: self.optimal_ec_min - current_ec,
                    reason: "EC 略低，建议增加施肥浓度".to_string()
                }
            }
            x if x > self.optimal_ec_max && x < self.warning_threshold_high => {
                ECRecommendation::DecreaseEC {
                    suggested_delta: current_ec - self.optimal_ec_max,
                    reason: "EC 偏高，建议降低施肥浓度或清水冲洗".to_string()
                }
            }
            x if x > self.warning_threshold_high => {
                ECRecommendation::ManualIntervention {
                    reason: format!(
                        "EC值({:.2})过高，可能造成盐害，请立即处理",
                        current_ec
                    ),
                    urgency: Urgency::Critical
                }
            }
            _ => ECRecommendation::NoAction
        }
    }
}
```

### 8.2 EC 值趋势分析

```rust
pub struct ECT rends {
    pub readings: Vec<(DateTime<Utc>, f64)>,
    pub period_hours: u32,
}

impl ECT rends {
    /// 分析 EC 变化趋势
    pub fn analyze(&self) -> ECTrend {
        if self.readings.len() < 3 {
            return ECTrend::InsufficientData;
        }
        
        // 计算斜率
        let slope = self.calculate_slope();
        
        match slope {
            s if s > 0.1 => ECTrend::Rising,
            s if s < -0.1 => ECTrend::Falling,
            _ => ECTrend::Stable
        }
    }
}
```

---

## 九、API 扩展

### 9.1 新增端点

```rust
// agri-server/src/ai_routes.rs

// 环境评估
POST /api/v1/ai/assess
{
  "area_id": "zone-1",
  "include_weather": true
}
→ {
  "assessment_id": "uuid",
  "scores": {
    "overall": 78,
    "soil_temp": 85,
    "soil_moisture": 72,
    "ec": 90
  },
  "pest_risks": [...],
  "recommendations": [...],
  "weather_impact": {...}
}

// 紧急情况触发
GET /api/v1/ai/emergency/status
→ {
  "active_emergencies": [],
  "weather_alerts": [
    {
      "type": "heavy_rain",
      "probability": 0.8,
      "eta_hours": 4
    }
  ],
  "night_mode_active": false
}

// 知识库查询
GET /api/v1/ai/knowledge/search?query=番茄灰霉病
GET /api/v1/ai/knowledge/cases?area_id=zone-1&limit=10

// 知识库更新（AI 学习）
POST /api/v1/ai/knowledge/cases
{
  "case_data": {...},
  "outcome": "success",
  "effectiveness": 5
}

// 卷膜器配置
GET /api/v1/ai/ventilation/config/{area_id}
POST /api/v1/ai/ventilation/calibrate/{device_id}

// EC 值分析
GET /api/v1/ai/ec/analyze/{area_id}
→ {
  "current_ec": 2.1,
  "trend": "stable",
  "recommendation": "NoAction"
}

// 控制命令（带紧急情况检查）
POST /api/v1/ai/control/ventilation
{
  "area_id": "zone-1",
  "vent_type": "top",
  "target_percent": 80,
  "reason": "降温"
}
→ {
  "command_id": "uuid",
  "status": "executed",  // 或 "pending_approval"
  "emergency_overridden": false,
  "message": "..."
}
```

---

## 十、实施计划

### Phase 1: 基础设施 (Week 1-2)
- [ ] 创建数据库表（knowledge, emergency_rules, greenhouse_config）
- [ ] 初始化作物知识库基础数据
- [ ] 搭建 AI 模块框架

### Phase 2: 紧急保护系统 (Week 2-3)
- [ ] 实现紧急检测规则引擎
- [ ] 大风/暴雨自动关闭逻辑
- [ ] 降雪保护 + 人工通知
- [ ] 夜间模式配置

### Phase 3: 环境评估 (Week 3-4)
- [ ] 环境评分算法
- [ ] 病虫害风险预测
- [ ] 调控建议生成

### Phase 4: 设备控制 (Week 4-5)
- [ ] 卷膜器量程学习流程
- [ ] 通风控制逻辑
- [ ] EC 值监测

### Phase 5: 知识积累 (持续)
- [ ] 案例自动记录
- [ ] RAG 检索优化
- [ ] 向量数据库集成

---

## 附录 A: Obsidian 知识库笔记模板

### A.1 作物知识笔记模板

```markdown
---
type: crop-profile
id: crop-{name}-{uuid}
name: {作物名称}
variety: {品种}
created: {日期}
tags:
  - 作物
  - 知识库
  - {生长阶段}
---

# {作物名称} ({品种})

## 环境参数

### 土壤温度
- 最低: {min}°C
- 最适: {optimal}°C  
- 最高: {max}°C

### 土壤湿度
- 最低: {min}%
- 最适: {optimal}%
- 最高: {max}%

### EC 值
- 最低: {min}
- 最适: {optimal}
- 最高: {max}

## 生长阶段

| 阶段 | 时长 | 环境要求 |
|------|------|----------|
| 苗期 | X天 | ... |
| 生长期 | X天 | ... |
| 开花期 | X天 | ... |
| 结果期 | X天 | ... |

## 通风偏好

- 偏好程度: [[高/中/低]]
- 风敏感度: {0-1}

## 相关病虫害

- [[灰霉病]] - 高湿度易发
- [[白粉病]] - 干燥通风差易发

## 调控案例

- [[2026-05-18-高温调控]] - 夏季降温经验
- [[2026-05-20-越冬管理]] - 冬季保温经验

## 备注

<!-- 人工补充区域 -->
```

### A.2 病虫害知识笔记模板

```markdown
---
type: pest-knowledge
id: pest-{name}-{uuid}
name: {病虫害名称}
crops: [{影响的作物}]
severity: {low/medium/high/critical}
created: {日期}
tags:
  - 病虫害
  - 知识库
---

# {病虫害名称}

## 基本信息

- **危害对象**: {作物类型}
- **严重程度**: [[{severity}]] 
- **是否紧急**: {是/否}

## 触发条件

| 参数 | 条件 |
|------|------|
| 温度 | {min} - {max}°C |
| 湿度 | > {min}% |
| 持续时间 | > {hours}小时 |

## 症状表现

{文字描述}

## 预防措施

{预防方法}

## 治疗方法

{治疗方法}

## 紧急处理

{如果是紧急病虫害}

## 相关案例

- [[zone-1/2026-05-18-高湿预警]] - 发现和处理记录

## 知识来源

- 来源: {专家/文献/经验}
- 置信度: {0.8}
```

### A.3 调控案例笔记模板

```markdown
---
type: control-case
id: case-{date}-{uuid}
area_id: {区域ID}
crop: {作物}
date: {日期}
outcome: {success/partial/failed}
effectiveness: {1-5}
tags:
  - 调控案例
  - {区域}
---

# {日期} - {调控类型}

## 区域信息

- 区域: [[{区域ID}]]
- 作物: [[{作物名称}]]

## 初始环境

| 参数 | 数值 | 评分 |
|------|------|------|
| 土壤温度 | {value}°C | {score} |
| 土壤湿度 | {value}% | {score} |
| EC值 | {value} | {score} |
| 总体评分 | {overall} | {score} |

## 触发原因

{为什么需要调控}

## 采取行动

```yaml
actions:
  - device: {设备}
    command: {命令}
    target: {目标值}
```

## 执行结果

### 执行后环境

| 参数 | 数值 | 评分 |
|------|------|------|
| 土壤温度 | {value}°C | {score} |
| ... | ... | ... |

### 恢复时间

{xx} 分钟后恢复到舒适区间

## 效果评估

- **评分**: [[{effectiveness}]]/5
- **总体结果**: [[{outcome}]]
- **健康分提升**: +{score}分

## 经验总结

<!-- AI 和人工总结 -->

## 相关知识

- [[{病虫害名}]] - 触发条件匹配
- [[{作物名}]] - 环境偏好参考

## 附录 B: 病虫害知识示例

```json
{
  "id": "pest-001",
  "name": "灰霉病",
  "trigger_conditions": {
    "humidity_min": 80,
    "temp_min": 15,
    "temp_max": 25,
    "duration_hours": 6
  },
  "severity": "high",
  "prevention": "加强通风降湿",
  "emergency_action": "开启最大通风"
}
```

## 附录 C: 气象规则示例

```yaml
wind_storm:
  threshold: 40 km/h
  action: close_top_vent
  requires_confirmation: false
  contact: true
  urgency: critical
  night_alert: true

heavy_rain:
  threshold: 10 mm/h
  action: close_top_vent
  requires_confirmation: false
  contact: true
  urgency: high

snow_risk:
  condition: temp < 3 AND snow_probability > 0.6
  action: close_all_vents
  pause_auto_mode: true
  contact: true
  urgency: CRITICAL
  message: |
    ⚠️ 降雪风险预警！
    请立即前往现场处理，防止积雪压垮大棚！
```

---

**文档版本历史**
| 版本 | 日期 | 说明 |
|------|------|------|
| v1.0 | 2026-05-18 | 初始版本，包含完整架构设计 |