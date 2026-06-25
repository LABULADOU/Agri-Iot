CREATE TABLE IF NOT EXISTS farm_operations (
    id TEXT PRIMARY KEY,
    area_id TEXT NOT NULL REFERENCES areas(id) ON DELETE CASCADE,
    log_date TEXT NOT NULL,
    log_time TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL,
    content TEXT NOT NULL,
    operator TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'completed' CHECK (status IN ('planned', 'in_progress', 'completed', 'cancelled')),
    weather TEXT NOT NULL DEFAULT '',
    crop_status TEXT NOT NULL DEFAULT '',
    notes TEXT NOT NULL DEFAULT '',
    details TEXT NOT NULL DEFAULT '{}',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS farm_operation_templates (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    category TEXT NOT NULL,
    details TEXT NOT NULL DEFAULT '{}',
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_farm_ops_area_date ON farm_operations(area_id, log_date);
CREATE INDEX IF NOT EXISTS idx_farm_ops_category ON farm_operations(category);
CREATE INDEX IF NOT EXISTS idx_farm_op_templates_category ON farm_operation_templates(category);
