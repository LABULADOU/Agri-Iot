CREATE TABLE IF NOT EXISTS mixing_presets (
    id TEXT PRIMARY KEY,
    crop_id TEXT,
    mix_type TEXT NOT NULL CHECK (mix_type IN ('fertilizer', 'pesticide')),
    stage_key TEXT NOT NULL DEFAULT '',
    name TEXT NOT NULL,
    items TEXT NOT NULL DEFAULT '[]',
    dilution TEXT NOT NULL DEFAULT '',
    dosage_per_unit TEXT NOT NULL DEFAULT '',
    water_volume TEXT NOT NULL DEFAULT '',
    ec_target REAL,
    safety_interval_days INTEGER NOT NULL DEFAULT 0,
    note TEXT NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS mixing_recipes (
    id TEXT PRIMARY KEY,
    area_id TEXT NOT NULL,
    crop_batch_id TEXT,
    mix_type TEXT NOT NULL CHECK (mix_type IN ('fertilizer', 'pesticide')),
    stage_key TEXT NOT NULL DEFAULT '',
    growth_days INTEGER NOT NULL DEFAULT 0,
    input TEXT NOT NULL DEFAULT '{}',
    result TEXT NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'generated' CHECK (status IN ('generated', 'applied')),
    farm_op_id TEXT NOT NULL DEFAULT '',
    warnings TEXT NOT NULL DEFAULT '[]',
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_mixing_presets_crop ON mixing_presets(crop_id, mix_type);
CREATE INDEX IF NOT EXISTS idx_mixing_recipes_area ON mixing_recipes(area_id);
CREATE INDEX IF NOT EXISTS idx_mixing_recipes_created ON mixing_recipes(created_at);