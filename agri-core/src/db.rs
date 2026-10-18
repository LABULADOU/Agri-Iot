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
    ensure_labor_schema(pool).await?;
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

// 旧版 labor_records 使用 team / paid_status INTEGER，与当前代码
// (category + worker + paid_status TEXT) 不兼容。015 迁移的
// CREATE TABLE IF NOT EXISTS 遇到已存在的旧表会跳过，故在此幂等重建。
async fn ensure_labor_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let has_legacy: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM pragma_table_info('labor_records') WHERE name = 'team'",
    )
    .fetch_one(pool)
    .await?;

    if has_legacy.0 == 0 {
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    sqlx::query("DROP INDEX IF EXISTS idx_labor_log_date").execute(&mut *tx).await?;
    sqlx::query("DROP INDEX IF EXISTS idx_labor_area_id").execute(&mut *tx).await?;
    sqlx::query("DROP TABLE IF EXISTS labor_records_legacy").execute(&mut *tx).await?;
    sqlx::query("ALTER TABLE labor_records RENAME TO labor_records_legacy")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "CREATE TABLE labor_records (
            id TEXT PRIMARY KEY,
            log_date TEXT NOT NULL,
            area_id TEXT,
            category TEXT NOT NULL,
            worker TEXT NOT NULL DEFAULT '',
            worker_count INTEGER NOT NULL DEFAULT 1,
            work_hours REAL NOT NULL DEFAULT 0,
            rate REAL NOT NULL DEFAULT 0,
            amount REAL NOT NULL DEFAULT 0,
            paid_status TEXT NOT NULL DEFAULT 'unpaid',
            operator TEXT NOT NULL DEFAULT '',
            notes TEXT NOT NULL DEFAULT '',
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO labor_records
            (id, log_date, area_id, category, worker, worker_count, work_hours, rate, amount, paid_status, operator, notes, created_at, updated_at)
         SELECT id, log_date, area_id, '其他', COALESCE(team, ''), worker_count,
                COALESCE(work_hours, 1.0), rate, amount,
                CASE WHEN paid_status = 1 OR paid_status = 'paid' THEN 'paid' ELSE 'unpaid' END,
                COALESCE(operator, ''), COALESCE(notes, ''),
                COALESCE(created_at, 0), COALESCE(updated_at, 0)
         FROM labor_records_legacy",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query("DROP TABLE labor_records_legacy").execute(&mut *tx).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_labor_log_date ON labor_records (log_date)")
        .execute(&mut *tx)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_labor_area_id ON labor_records (area_id)")
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    println!("[db] Rebuilt labor_records to current schema (team→worker, paid_status INTEGER→TEXT)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn pool_with_legacy_labor() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE labor_records (
                id TEXT PRIMARY KEY,
                log_date TEXT NOT NULL,
                area_id TEXT,
                team TEXT NOT NULL,
                worker_count INTEGER NOT NULL,
                work_hours REAL DEFAULT 1.0,
                rate REAL NOT NULL,
                amount REAL NOT NULL,
                paid_status INTEGER DEFAULT 0,
                operator TEXT,
                notes TEXT,
                created_at INTEGER,
                updated_at INTEGER
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("CREATE INDEX idx_labor_log_date ON labor_records (log_date)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO labor_records VALUES
                ('l1','2026-09-01',NULL,'何琼弟班组',3,8.0,20.0,480.0,0,'许晗','-',1,1),
                ('l2','2026-09-02',NULL,'李琼芬班组',2,4.0,25.0,200.0,1,'许晗','',2,2)",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    #[tokio::test]
    async fn ensure_labor_schema_rebuilds_legacy_table() {
        let pool = pool_with_legacy_labor().await;
        ensure_labor_schema(&pool).await.unwrap();

        let cat: String = sqlx::query_scalar("SELECT category FROM labor_records WHERE id='l1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        let worker: String = sqlx::query_scalar("SELECT worker FROM labor_records WHERE id='l1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        let paid: String = sqlx::query_scalar("SELECT paid_status FROM labor_records WHERE id='l1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        let paid2: String = sqlx::query_scalar("SELECT paid_status FROM labor_records WHERE id='l2'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(cat, "其他");
        assert_eq!(worker, "何琼弟班组");
        assert_eq!(paid, "unpaid");
        assert_eq!(paid2, "paid");

        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM labor_records")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(n, 2);

        // 幂等：再次运行不改变数据，也不再重建
        ensure_labor_schema(&pool).await.unwrap();
        let n2: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM labor_records")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(n2, 2);
    }

    #[tokio::test]
    async fn ensure_labor_schema_noop_on_current_schema() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE labor_records (
                id TEXT PRIMARY KEY, log_date TEXT NOT NULL, area_id TEXT,
                category TEXT NOT NULL, worker TEXT NOT NULL DEFAULT '',
                worker_count INTEGER NOT NULL DEFAULT 1, work_hours REAL NOT NULL DEFAULT 0,
                rate REAL NOT NULL DEFAULT 0, amount REAL NOT NULL DEFAULT 0,
                paid_status TEXT NOT NULL DEFAULT 'unpaid', operator TEXT NOT NULL DEFAULT '',
                notes TEXT NOT NULL DEFAULT '', created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        ensure_labor_schema(&pool).await.unwrap();
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM labor_records")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(n, 0);
    }
}
