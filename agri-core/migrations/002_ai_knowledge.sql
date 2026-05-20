-- AI 决策系统：知识库与评估数据表
-- 依赖：001_init.sql（areas, devices, sensor_readings 等基础表）

-- 1. 作物知识库
CREATE TABLE IF NOT EXISTS crop_profiles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    variety TEXT,
    growth_stages TEXT,
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
    ec_min REAL,
    ec_max REAL,
    ec_optimal REAL,
    ventilation_preference TEXT CHECK (ventilation_preference IN ('high','medium','low')),
    wind_sensitivity REAL,
    embedding_id TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);

-- 2. 病虫害知识库
CREATE TABLE IF NOT EXISTS pest_knowledge (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    crop_types TEXT,
    trigger_conditions TEXT,
    symptoms TEXT,
    severity TEXT CHECK (severity IN ('low','medium','high','critical')),
    prevention TEXT,
    treatment TEXT,
    medication TEXT,
    is_emergency INTEGER NOT NULL DEFAULT 0,
    emergency_action TEXT,
    source TEXT,
    confidence REAL NOT NULL DEFAULT 0.8,
    embedding_id TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);

-- 3. 调控案例库
CREATE TABLE IF NOT EXISTS control_cases (
    id TEXT PRIMARY KEY,
    area_id TEXT REFERENCES areas(id) ON DELETE SET NULL,
    crop_profile_id TEXT,
    situation TEXT,
    weather_forecast TEXT,
    action_taken TEXT,
    manual_override INTEGER NOT NULL DEFAULT 0,
    outcome TEXT CHECK (outcome IN ('success','partial','failed')),
    effect_rating INTEGER CHECK (effect_rating BETWEEN 1 AND 5),
    health_improvement REAL,
    action_duration_minutes INTEGER,
    recovery_time_minutes INTEGER,
    notes TEXT,
    timestamp INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    embedding_id TEXT
);

-- 4. 案例有效性追踪
CREATE TABLE IF NOT EXISTS case_effectiveness (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    case_id TEXT NOT NULL REFERENCES control_cases(id) ON DELETE CASCADE,
    assessment_time INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    soil_temp_score REAL,
    soil_moisture_score REAL,
    pest_occurred INTEGER NOT NULL DEFAULT 0,
    notes TEXT
);

-- 5. 气象知识库
CREATE TABLE IF NOT EXISTS weather_knowledge (
    id TEXT PRIMARY KEY,
    condition_type TEXT NOT NULL CHECK (condition_type IN ('wind','rain','snow','storm','heat','frost')),
    thresholds TEXT,
    protection_rules TEXT,
    time_constraints TEXT,
    contact_required INTEGER NOT NULL DEFAULT 0,
    contact_urgency TEXT CHECK (contact_urgency IN ('normal','urgent','critical')),
    contact_message TEXT,
    notes TEXT,
    embedding_id TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);

-- 6. 大棚设备配置
CREATE TABLE IF NOT EXISTS greenhouse_config (
    id TEXT PRIMARY KEY,
    area_id TEXT NOT NULL REFERENCES areas(id) ON DELETE CASCADE,
    top_vent_min_percent REAL NOT NULL DEFAULT 0,
    top_vent_max_percent REAL NOT NULL DEFAULT 100,
    top_vent_current_percent REAL NOT NULL DEFAULT 0,
    top_vent_device_id TEXT,
    side_vent_min_percent REAL NOT NULL DEFAULT 0,
    side_vent_max_percent REAL NOT NULL DEFAULT 100,
    side_vent_current_percent REAL NOT NULL DEFAULT 0,
    side_vent_device_id TEXT,
    irrigation_device_id TEXT,
    fertigation_device_id TEXT,
    emergency_contact_name TEXT,
    emergency_contact_phone TEXT,
    top_vent_calibrated INTEGER NOT NULL DEFAULT 0,
    side_vent_calibrated INTEGER NOT NULL DEFAULT 0,
    calibration_date INTEGER,
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_greenhouse_config_area ON greenhouse_config(area_id);

-- 7. 传感器配置
CREATE TABLE IF NOT EXISTS sensor_config (
    id TEXT PRIMARY KEY,
    area_id TEXT NOT NULL REFERENCES areas(id) ON DELETE CASCADE,
    sensor_type TEXT NOT NULL CHECK (sensor_type IN ('soil_temp','soil_moisture','ec','air_temp','air_humidity')),
    device_id TEXT REFERENCES devices(id) ON DELETE SET NULL,
    calibration_offset REAL NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    last_reading INTEGER,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);

-- 8. 气象数据
CREATE TABLE IF NOT EXISTS weather_data (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    area_id TEXT REFERENCES areas(id) ON DELETE SET NULL,
    source TEXT NOT NULL CHECK (source IN ('api','local','forecast')),
    temperature REAL,
    humidity REAL,
    wind_speed REAL,
    wind_direction TEXT,
    precipitation REAL,
    snow_probability REAL,
    uv_index REAL,
    forecast_hour INTEGER,
    timestamp INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);

CREATE INDEX IF NOT EXISTS idx_weather_data_area ON weather_data(area_id);
CREATE INDEX IF NOT EXISTS idx_weather_data_timestamp ON weather_data(timestamp);

-- 9. 环境评估记录
CREATE TABLE IF NOT EXISTS env_assessments (
    id TEXT PRIMARY KEY,
    area_id TEXT REFERENCES areas(id) ON DELETE CASCADE,
    crop_profile_id TEXT,
    timestamp INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    overall_score REAL,
    soil_temp_score REAL,
    soil_moisture_score REAL,
    ec_score REAL,
    air_temp_score REAL,
    air_humidity_score REAL,
    deviations TEXT,
    pest_risks TEXT,
    recommendations TEXT,
    weather_impact TEXT,
    is_emergency INTEGER NOT NULL DEFAULT 0,
    emergency_type TEXT
);

CREATE INDEX IF NOT EXISTS idx_env_assessments_area ON env_assessments(area_id);

-- 10. 知识库更新日志
CREATE TABLE IF NOT EXISTS kb_update_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    update_type TEXT NOT NULL CHECK (update_type IN ('case_added','knowledge_curated','feedback_received')),
    source TEXT NOT NULL CHECK (source IN ('manual','auto','ai_review')),
    content_summary TEXT,
    effectiveness_score REAL,
    timestamp INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);
