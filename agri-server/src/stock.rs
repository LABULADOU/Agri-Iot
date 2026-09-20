use sqlx::Transaction;
use uuid::Uuid;

// ==================== 用量解析 ====================

/// 从自由文本用量中解析出 (数量, 单位)。
/// 兼容 "10 kg"、"300g/株"、"5袋"、"0.5"；稀释倍数类（含"倍"/"ppm"）视为无效返回 None。
pub fn parse_amount(text: &str) -> Option<(f64, String)> {
    None
}

/// 归一化单位，方便跨单位换算与比较
fn normalize_unit(unit: &str) -> String {
    String::new()
}

/// 将数量从 from 单位换算到 to 单位（kg↔g、kg↔t、L↔ml）。无法换算返回 None。
fn convert_amount(quantity: f64, from: &str, to: &str) -> Option<f64> {
    None
}

#[derive(Debug, Clone, PartialEq)]
pub struct UsageItem {
    pub name: String,
    pub amount: f64,
    pub unit: String,
    pub alt_names: Vec<String>,
}

/// 从农事操作 details 中提取待扣减条目（施肥读 items[].name+amount；打药读 brand/ingredient+usage）
pub fn parse_usage_items(category: &str, details: &serde_json::Value) -> Vec<UsageItem> {
    Vec::new()
}

// ==================== 出库联动 ====================

/// 对一次农事操作执行自动出库（须在事务内调用），返回 warnings。
/// 行为与配肥配药一键应用一致：库存不足按实际扣减、未找到物品/单位不匹配仅警告。
pub async fn apply_op_deductions(
    tx: &mut Transaction<'_, sqlx::Sqlite>,
    op_id: &str,
    category: &str,
    details: &serde_json::Value,
    operator: &str,
) -> Result<Vec<String>, sqlx::Error> {
    Ok(Vec::new())
}

/// 回滚某农事操作关联的出库流水：恢复库存并删除流水（须在事务内调用）
pub async fn revert_op_deductions(
    tx: &mut Transaction<'_, sqlx::Sqlite>,
    op_id: &str,
) -> Result<(), sqlx::Error> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::{Row, SqlitePool};

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let schema = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../agri-core/migrations/012_inventory.sql")).unwrap();
        for stmt in schema.split(';').filter(|s| !s.trim().is_empty()) {
            sqlx::query(stmt).execute(&pool).await.unwrap();
        }
        pool
    }

    async fn seed_item(pool: &SqlitePool, name: &str, unit: &str, stock: f64) {
        sqlx::query(
            "INSERT INTO inventory_items (id, name, category, unit, price, stock, warning_threshold, created_at, updated_at) VALUES (?, ?, 'fertilizer', ?, 0, ?, 0, 0, 0)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(name)
        .bind(unit)
        .bind(stock)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn get_stock(pool: &SqlitePool, name: &str) -> f64 {
        let row = sqlx::query("SELECT stock FROM inventory_items WHERE name = ?")
            .bind(name)
            .fetch_one(pool)
            .await
            .unwrap();
        row.try_get("stock").unwrap()
    }

    async fn txn_count(pool: &SqlitePool, op_id: &str) -> i64 {
        let row = sqlx::query(
            "SELECT COUNT(*) AS c FROM inventory_transactions WHERE related_type = 'farm_operation' AND related_id = ?",
        )
        .bind(op_id)
        .fetch_one(pool)
        .await
        .unwrap();
        row.try_get("c").unwrap()
    }

    #[test]
    fn test_parse_amount_basic() {
        assert_eq!(parse_amount("10 kg"), Some((10.0, "kg".to_string())));
        assert_eq!(parse_amount("300g/株"), Some((300.0, "g".to_string())));
        assert_eq!(parse_amount("5袋"), Some((5.0, "袋".to_string())));
        assert_eq!(parse_amount("0.5"), Some((0.5, String::new())));
        assert_eq!(parse_amount(" 2.5公斤 "), Some((2.5, "公斤".to_string())));
    }

    #[test]
    fn test_parse_amount_rejects() {
        assert_eq!(parse_amount("1500倍液"), None);
        assert_eq!(parse_amount("适量"), None);
        assert_eq!(parse_amount(""), None);
    }

    #[test]
    fn test_parse_usage_items_fertilizer() {
        let details = serde_json::json!({
            "items": [
                {"name": "尿素", "amount": "10 kg", "n": ""},
                {"name": "硝酸钙", "amount": "300g/株"}
            ]
        });
        let items = parse_usage_items("施肥", &details);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], UsageItem { name: "尿素".into(), amount: 10.0, unit: "kg".into(), alt_names: vec![] });
        assert_eq!(items[1], UsageItem { name: "硝酸钙".into(), amount: 300.0, unit: "g".into(), alt_names: vec![] });
    }

    #[test]
    fn test_parse_usage_items_pesticide() {
        let details = serde_json::json!({
            "items": [{"ingredient": "嘧霉胺", "brand": "巴斯夫", "usage": 30, "usage_unit": "ml"}]
        });
        let items = parse_usage_items("打药", &details);
        assert_eq!(items.len(), 1);
        let it = &items[0];
        assert_eq!(it.name, "巴斯夫");
        assert_eq!(it.amount, 30.0);
        assert_eq!(it.unit, "ml");
        assert_eq!(it.alt_names, vec!["嘧霉胺", "巴斯夫"]);
    }

    #[test]
    fn test_parse_usage_items_non_relevant_category_returns_empty() {
        let details = serde_json::json!({"items": [{"name": "尿素", "amount": "10 kg"}]});
        assert!(parse_usage_items("灌溉", &details).is_empty());
        // mixing 生成的 details（无顶层 items 数组）
        let mixing = serde_json::json!({"recipe_id": "r1", "result": {"plan": {"items": [{"name": "尿素", "amount": 10}]}}});
        assert!(parse_usage_items("施肥", &mixing).is_empty());
    }

    #[tokio::test]
    async fn test_apply_deductions_deducts_and_records() {
        let pool = setup_db().await;
        seed_item(&pool, "尿素", "kg", 100.0).await;
        let mut tx = pool.begin().await.unwrap();
        let details = serde_json::json!({"items": [{"name": "尿素", "amount": "10 kg"}]});
        let warnings = apply_op_deductions(&mut tx, "op-1", "施肥", &details, "张三").await.unwrap();
        tx.commit().await.unwrap();
        assert!(warnings.is_empty());
        assert_eq!(get_stock(&pool, "尿素").await, 90.0);
        assert_eq!(txn_count(&pool, "op-1").await, 1);
    }

    #[tokio::test]
    async fn test_apply_deductions_insufficient_clamped() {
        let pool = setup_db().await;
        seed_item(&pool, "尿素", "kg", 5.0).await;
        let mut tx = pool.begin().await.unwrap();
        let details = serde_json::json!({"items": [{"name": "尿素", "amount": "10 kg"}]});
        let warnings = apply_op_deductions(&mut tx, "op-1", "施肥", &details, "").await.unwrap();
        tx.commit().await.unwrap();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("库存不足"));
        assert_eq!(get_stock(&pool, "尿素").await, 0.0);
    }

    #[tokio::test]
    async fn test_apply_deductions_unit_mismatch_skips() {
        let pool = setup_db().await;
        seed_item(&pool, "尿素", "kg", 100.0).await;
        let mut tx = pool.begin().await.unwrap();
        let details = serde_json::json!({"items": [{"name": "尿素", "amount": "10 ml"}]});
        let warnings = apply_op_deductions(&mut tx, "op-1", "施肥", &details, "").await.unwrap();
        tx.commit().await.unwrap();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("单位"));
        assert_eq!(get_stock(&pool, "尿素").await, 100.0);
        assert_eq!(txn_count(&pool, "op-1").await, 0);
    }

    #[tokio::test]
    async fn test_apply_deductions_not_found_warns() {
        let pool = setup_db().await;
        let mut tx = pool.begin().await.unwrap();
        let details = serde_json::json!({"items": [{"name": "不存在的东西", "amount": "10 kg"}]});
        let warnings = apply_op_deductions(&mut tx, "op-1", "施肥", &details, "").await.unwrap();
        tx.commit().await.unwrap();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("未找到"));
    }

    #[tokio::test]
    async fn test_apply_skips_irrelevant_category() {
        let pool = setup_db().await;
        seed_item(&pool, "尿素", "kg", 100.0).await;
        let mut tx = pool.begin().await.unwrap();
        let details = serde_json::json!({"items": [{"name": "尿素", "amount": "10 kg"}]});
        let warnings = apply_op_deductions(&mut tx, "op-1", "灌溉", &details, "").await.unwrap();
        tx.commit().await.unwrap();
        assert!(warnings.is_empty());
        assert_eq!(txn_count(&pool, "op-1").await, 0);
    }

    #[tokio::test]
    async fn test_apply_deductions_falls_back_to_ingredient_match() {
        let pool = setup_db().await;
        seed_item(&pool, "巴斯夫凯泽", "ml", 200.0).await;
        let mut tx = pool.begin().await.unwrap();
        let details = serde_json::json!({
            "items": [{"ingredient": "凯泽", "brand": "巴斯夫凯泽", "usage": 30, "usage_unit": "ml"}]
        });
        let warnings = apply_op_deductions(&mut tx, "op-1", "打药", &details, "").await.unwrap();
        tx.commit().await.unwrap();
        assert!(warnings.is_empty());
        assert_eq!(get_stock(&pool, "巴斯夫凯泽").await, 170.0);
    }

    #[tokio::test]
    async fn test_revert_deductions_restores_stock_and_deletes_txns() {
        let pool = setup_db().await;
        seed_item(&pool, "尿素", "kg", 100.0).await;
        let mut tx = pool.begin().await.unwrap();
        let details = serde_json::json!({"items": [{"name": "尿素", "amount": "10 kg"}]});
        apply_op_deductions(&mut tx, "op-1", "施肥", &details, "").await.unwrap();
        revert_op_deductions(&mut tx, "op-1").await.unwrap();
        tx.commit().await.unwrap();
        assert_eq!(get_stock(&pool, "尿素").await, 100.0);
        assert_eq!(txn_count(&pool, "op-1").await, 0);
    }
}