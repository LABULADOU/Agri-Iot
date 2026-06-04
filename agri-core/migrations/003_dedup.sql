-- Add seq column for MQTT QoS dedup
-- seq is an incrementing counter from the device, nullable for HTTP legacy clients
ALTER TABLE sensor_readings ADD COLUMN seq INTEGER;

-- Partial unique index: only applies when seq IS NOT NULL (MQTT clients)
CREATE UNIQUE INDEX IF NOT EXISTS idx_readings_dedup ON sensor_readings(device_id, metric, seq) WHERE seq IS NOT NULL;
