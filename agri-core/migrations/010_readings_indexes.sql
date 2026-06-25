-- Add composite indexes on sensor_readings for query performance.
-- Without these, queries filtering by device_id + timestamp range
-- fall back to full table scan + sort.

-- Covers list_readings: WHERE device_id=? ORDER BY timestamp DESC
CREATE INDEX IF NOT EXISTS idx_readings_device_ts
    ON sensor_readings(device_id, timestamp);

-- Covers readings_aggregate: WHERE device_id=? AND metric=? AND timestamp BETWEEN
CREATE INDEX IF NOT EXISTS idx_readings_device_metric_ts
    ON sensor_readings(device_id, metric, timestamp);
