-- 区域表
CREATE TABLE IF NOT EXISTS zones (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    location TEXT NOT NULL,
    crop_type TEXT NOT NULL,
    comfort_config TEXT NOT NULL,
    node_ids TEXT NOT NULL DEFAULT '[]',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- 传感器节点表
CREATE TABLE IF NOT EXISTS sensor_nodes (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    zone_id TEXT NOT NULL,
    has_irrigation INTEGER NOT NULL DEFAULT 0,
    has_side_vent INTEGER NOT NULL DEFAULT 0,
    has_roof_vent INTEGER NOT NULL DEFAULT 0,
    vent_range TEXT NOT NULL DEFAULT '{"min": 0, "max": 100}',
    status TEXT NOT NULL DEFAULT 'offline' CHECK (status IN ('online', 'offline', 'error')),
    last_seen INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (zone_id) REFERENCES zones(id) ON DELETE CASCADE
);

CREATE INDEX idx_nodes_zone_id ON sensor_nodes(zone_id);
CREATE INDEX idx_nodes_status ON sensor_nodes(status);

-- 积温表
CREATE TABLE IF NOT EXISTS accumulated_temps (
    id TEXT PRIMARY KEY,
    zone_id TEXT NOT NULL,
    date TEXT NOT NULL,
    accumulated REAL NOT NULL DEFAULT 0,
    threshold REAL NOT NULL DEFAULT 10,
    UNIQUE(zone_id, date),
    FOREIGN KEY (zone_id) REFERENCES zones(id) ON DELETE CASCADE
);

CREATE INDEX idx_acc_temp_zone_date ON accumulated_temps(zone_id, date);