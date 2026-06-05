-- Fix migration 004: the new idx_readings_dedup was never created
-- because 003 had already created an index with the same name.
-- 004 used IF NOT EXISTS which silently skipped creation.

-- Drop the old 3-column index (from migration 003)
DROP INDEX IF EXISTS idx_readings_dedup;

-- Re-create with 4 columns including boot_id (as intended by 004)
CREATE UNIQUE INDEX IF NOT EXISTS idx_readings_dedup
  ON sensor_readings(device_id, metric, seq, boot_id)
  WHERE seq IS NOT NULL AND boot_id IS NOT NULL;
