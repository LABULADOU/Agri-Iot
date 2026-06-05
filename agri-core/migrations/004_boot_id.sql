-- Add boot_id for cross-session dedup
-- boot_id is a random string generated once per device boot/firmware flash
ALTER TABLE sensor_readings ADD COLUMN boot_id TEXT;

-- Legacy dedup for devices without boot_id (seq-only)
-- Kept for backward compatibility with old ESP32 firmware
CREATE UNIQUE INDEX IF NOT EXISTS idx_readings_dedup_legacy
  ON sensor_readings(device_id, metric, seq)
  WHERE seq IS NOT NULL AND boot_id IS NULL;

-- New dedup for devices with boot_id (seq + boot_id)
-- Combined with seq, uniquely identifies a reading across device lifetimes
-- When a device is reflashed, boot_id changes, preventing seq collision
CREATE UNIQUE INDEX IF NOT EXISTS idx_readings_dedup
  ON sensor_readings(device_id, metric, seq, boot_id)
  WHERE seq IS NOT NULL AND boot_id IS NOT NULL;
