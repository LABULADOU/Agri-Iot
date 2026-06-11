CREATE TABLE IF NOT EXISTS entity_relations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_id TEXT NOT NULL,
    from_type TEXT NOT NULL,
    to_id TEXT NOT NULL,
    to_type TEXT NOT NULL,
    relation_type TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    UNIQUE(from_id, from_type, to_id, to_type, relation_type)
);

CREATE INDEX IF NOT EXISTS idx_er_from ON entity_relations(from_id, from_type);
CREATE INDEX IF NOT EXISTS idx_er_to ON entity_relations(to_id, to_type);

INSERT OR IGNORE INTO entity_relations (from_id, from_type, to_id, to_type, relation_type, created_at)
SELECT id, 'device', area_id, 'area', 'belongs_to', strftime('%s', 'now')
FROM devices WHERE area_id IS NOT NULL;
