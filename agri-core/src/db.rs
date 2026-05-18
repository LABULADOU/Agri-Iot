use sqlx::SqlitePool;
use std::time::Duration;

pub async fn create_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let pool = SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new()
            .filename(database_url)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(5)),
    )
    .await?;

    Ok(pool)
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::migrate!("./migrations").run(pool).await?;
    ensure_capabilities_column(pool).await?;
    ensure_unique_node_id(pool).await?;
    merge_duplicate_devices(pool).await?;
    Ok(())
}

async fn ensure_capabilities_column(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let has_column: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM pragma_table_info('devices') WHERE name = 'capabilities'"
    )
    .fetch_one(pool)
    .await?;

    if has_column.0 == 0 {
        sqlx::query("ALTER TABLE devices ADD COLUMN capabilities TEXT NOT NULL DEFAULT '[\"sensor\"]'")
            .execute(pool)
            .await?;
        println!("[db] Added capabilities column to devices table");
    }

    Ok(())
}

async fn ensure_unique_node_id(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // 先检查是否已存在 UNIQUE 索引
    let has_unique: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'idx_devices_node_id' AND sql LIKE '%UNIQUE%'"
    )
    .fetch_one(pool)
    .await?;

    if has_unique.0 == 0 {
        // 删除旧的普通索引（如果存在），创建 UNIQUE 索引
        sqlx::query("DROP INDEX IF EXISTS idx_devices_node_id").execute(pool).await?;
        sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS idx_devices_node_id ON devices(node_id)")
            .execute(pool)
            .await?;
        println!("[db] Created UNIQUE index on devices.node_id");
    }

    Ok(())
}

async fn merge_duplicate_devices(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // 将 command_log 中对 actuator 的引用重定向到同 node_id 的 sensor
    sqlx::query(
        "UPDATE command_log SET device_id = (
            SELECT d1.id FROM devices d1
            WHERE d1.node_id = (
                SELECT d2.node_id FROM devices d2 WHERE d2.id = command_log.device_id
            )
            AND d1.device_type = 'sensor'
            LIMIT 1
        )
        WHERE device_id IN (
            SELECT id FROM devices WHERE device_type = 'actuator'
            AND node_id IN (
                SELECT node_id FROM devices GROUP BY node_id HAVING COUNT(*) > 1
            )
        )"
    )
    .execute(pool)
    .await?;

    // 合并 capabilities
    sqlx::query(
        "UPDATE devices SET capabilities = '[\"sensor\",\"actuator\"]'
        WHERE id IN (
            SELECT d1.id FROM devices d1
            WHERE d1.device_type = 'sensor'
            AND EXISTS (
                SELECT 1 FROM devices d2
                WHERE d2.node_id = d1.node_id AND d2.device_type = 'actuator'
            )
        )"
    )
    .execute(pool)
    .await?;

    // 删除 actuator 重复记录
    sqlx::query(
        "DELETE FROM devices WHERE device_type = 'actuator'
        AND node_id IN (
            SELECT node_id FROM devices GROUP BY node_id HAVING COUNT(*) > 1
        )"
    )
    .execute(pool)
    .await?;

    Ok(())
}
