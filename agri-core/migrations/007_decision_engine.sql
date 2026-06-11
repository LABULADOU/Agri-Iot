-- Decision engine: decision log and notification config

CREATE TABLE IF NOT EXISTS decision_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    flow_name TEXT NOT NULL,
    node_id TEXT NOT NULL DEFAULT '',
    trigger TEXT NOT NULL DEFAULT '',
    outcome TEXT NOT NULL DEFAULT '',
    detail TEXT,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_decision_log_node ON decision_log(node_id, created_at);
CREATE INDEX IF NOT EXISTS idx_decision_log_time ON decision_log(created_at);
