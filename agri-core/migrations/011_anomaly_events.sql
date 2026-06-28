-- Anomaly events detected by the data anomaly detection engine.
-- Stores evidence from E1-E5 detectors for historical review and frontend display.

CREATE TABLE IF NOT EXISTS anomaly_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    node_id TEXT NOT NULL,
    metric TEXT NOT NULL,
    anomaly_type TEXT NOT NULL,   -- 'Dht22Fault', 'RateAnomaly', 'SpatialAnomaly', 'MetricSilent'
    severity TEXT NOT NULL DEFAULT 'warning',  -- 'Info', 'Warning', 'Critical'
    value_original REAL,
    message TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE INDEX idx_anomaly_device ON anomaly_events(device_id, created_at);
CREATE INDEX idx_anomaly_node ON anomaly_events(node_id, created_at);
CREATE INDEX idx_anomaly_type ON anomaly_events(anomaly_type, created_at);
