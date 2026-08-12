CREATE TABLE IF NOT EXISTS inventory_items (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    category TEXT NOT NULL CHECK (category IN ('seed', 'fertilizer', 'pesticide', 'materiel', 'other')),
    unit TEXT NOT NULL DEFAULT '',
    price REAL NOT NULL DEFAULT 0,
    stock REAL NOT NULL DEFAULT 0,
    warning_threshold REAL NOT NULL DEFAULT 0,
    batch_no TEXT NOT NULL DEFAULT '',
    expiry_date TEXT NOT NULL DEFAULT '',
    manufacturer TEXT NOT NULL DEFAULT '',
    specs TEXT NOT NULL DEFAULT '',
    notes TEXT NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS inventory_transactions (
    id TEXT PRIMARY KEY,
    item_id TEXT NOT NULL REFERENCES inventory_items(id) ON DELETE CASCADE,
    txn_type TEXT NOT NULL CHECK (txn_type IN ('in', 'out', 'adjust')),
    quantity REAL NOT NULL,
    operator TEXT NOT NULL DEFAULT '',
    related_type TEXT NOT NULL DEFAULT '',
    related_id TEXT NOT NULL DEFAULT '',
    note TEXT NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_inventory_items_category ON inventory_items(category);
CREATE INDEX IF NOT EXISTS idx_inventory_txn_item ON inventory_transactions(item_id);
CREATE INDEX IF NOT EXISTS idx_inventory_txn_created ON inventory_transactions(created_at);