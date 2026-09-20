-- 每日用工成本记录（labor_records）
CREATE TABLE IF NOT EXISTS labor_records (
    id TEXT PRIMARY KEY,
    log_date TEXT NOT NULL,
    area_id TEXT,
    category TEXT NOT NULL,
    worker TEXT NOT NULL DEFAULT '',
    worker_count INTEGER NOT NULL DEFAULT 1,
    work_hours REAL NOT NULL DEFAULT 0,
    rate REAL NOT NULL DEFAULT 0,
    amount REAL NOT NULL DEFAULT 0,
    paid_status TEXT NOT NULL DEFAULT 'unpaid',
    operator TEXT NOT NULL DEFAULT '',
    notes TEXT NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_labor_log_date ON labor_records (log_date);
CREATE INDEX IF NOT EXISTS idx_labor_area_id ON labor_records (area_id);