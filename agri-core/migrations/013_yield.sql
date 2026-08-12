CREATE TABLE IF NOT EXISTS harvests (
    id TEXT PRIMARY KEY,
    area_id TEXT NOT NULL REFERENCES areas(id) ON DELETE CASCADE,
    crop_batch_id TEXT REFERENCES crop_batches(id) ON DELETE SET NULL,
    harvest_date TEXT NOT NULL,
    quantity REAL NOT NULL,
    unit TEXT NOT NULL DEFAULT 'kg',
    grade TEXT NOT NULL DEFAULT '',
    price REAL NOT NULL DEFAULT 0,
    amount REAL NOT NULL DEFAULT 0,
    operator TEXT NOT NULL DEFAULT '',
    notes TEXT NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_harvests_area ON harvests(area_id);
CREATE INDEX IF NOT EXISTS idx_harvests_batch ON harvests(crop_batch_id);
CREATE INDEX IF NOT EXISTS idx_harvests_date ON harvests(harvest_date);