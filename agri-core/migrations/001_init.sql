CREATE TABLE IF NOT EXISTS areas (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS crops (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    comfort_config TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS crop_batches (
    id TEXT PRIMARY KEY,
    area_id TEXT NOT NULL REFERENCES areas(id) ON DELETE CASCADE,
    crop_id TEXT NOT NULL REFERENCES crops(id) ON DELETE CASCADE,
    plant_date INTEGER NOT NULL,
    expected_harvest_date INTEGER,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'harvested', 'failed')),
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS devices (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    node_id TEXT NOT NULL,
    device_type TEXT NOT NULL CHECK (device_type IN ('sensor', 'actuator')),
    status TEXT NOT NULL DEFAULT 'offline' CHECK (status IN ('online', 'offline', 'error')),
    config TEXT,
    area_id TEXT REFERENCES areas(id) ON DELETE SET NULL,
    comfort_config TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS sensor_readings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    metric TEXT NOT NULL,
    value REAL NOT NULL,
    unit TEXT NOT NULL,
    timestamp INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS rules (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    trigger_type TEXT NOT NULL CHECK (trigger_type IN ('schedule', 'condition')),
    conditions TEXT NOT NULL,
    actions TEXT NOT NULL,
    schedule TEXT,
    priority INTEGER NOT NULL DEFAULT 0,
    auto_execute INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS command_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    command TEXT NOT NULL,
    payload TEXT,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'sent', 'completed', 'failed', 'timeout')),
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_crop_batches_area ON crop_batches(area_id);
CREATE INDEX IF NOT EXISTS idx_crop_batches_crop ON crop_batches(crop_id);
CREATE INDEX IF NOT EXISTS idx_crop_batches_status ON crop_batches(status);
CREATE INDEX IF NOT EXISTS idx_devices_area ON devices(area_id);
CREATE INDEX IF NOT EXISTS idx_readings_device_metric ON sensor_readings(device_id, metric);
CREATE INDEX IF NOT EXISTS idx_readings_timestamp ON sensor_readings(timestamp);
CREATE INDEX IF NOT EXISTS idx_command_log_device ON command_log(device_id);
CREATE INDEX IF NOT EXISTS idx_command_log_status ON command_log(status);
