use sqlx::{Row, Transaction};
use uuid::Uuid;

// ==================== 用量解析 ====================

/// 从自由文本用量中解析出 (数量, 单位)。
/// 兼容 "10 kg"、"300g/株"、"5袋"、"0.5"；稀释倍数类（含"倍"/"ppm"）视为无效返回 None。
pub fn parse_amount(text: &str) -> Option<(f64, String)> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    let mut num_start: Option<usize> = None;
    let mut num_end = 0usize;
    let mut seen_dot = false;
    for (i, c) in t.char_indices() {
        if num_start.is_none() {
            if c.is_ascii_digit() || c == '.' {
                num_start = Some(i);
                num_end = i + c.len_utf8();
                if c == '.' {
                    seen_dot = true;
                }
            } else if !c.is_whitespace() {
                return None;
            }
        } else if c.is_ascii_digit() || (c == '.' && !seen_dot) {
            if c == '.' {
                seen_dot = true;
            }
            num_end = i + c.len_utf8();
        } else {
            break;
        }
    }
    let start = num_start?;
    let number: f64 = t[start..num_end].parse().ok()?;
    if number <= 0.0 {
        return None;
    }
    let unit_raw = t[num_end..].trim();
    if unit_raw.is_empty() {
        return Some((number, String::new()));
    }
    // 取第一个分隔符前的单位（如 "10 kg/亩"、"300g/株"）
    let unit = unit_raw
        .split(['/', '(', '（', ' ', '　'])
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    if unit.is_empty() {
        return Some((number, String::new()));
    }
    let norm = normalize_unit(&unit);
    if norm.contains("倍") || norm.contains("ppm") || norm.contains("%") {
        return None;
    }
    Some((number, norm))
}

/// 归一化单位，方便跨单位换算与比较
fn normalize_unit(unit: &str) -> String {
    let u = unit.trim().to_lowercase();
    match u.as_str() {
        "千克" | "公斤" | "kg" | "kgs" => "kg".to_string(),
        "克" | "g" | "gram" | "grams" => "g".to_string(),
        "吨" | "t" | "ton" => "t".to_string(),
        "升" | "l" | "liter" | "liters" => "L".to_string(),
        "毫升" | "ml" => "ml".to_string(),
        _ => u,
    }
}

/// 将数量从 from 单位换算到 to 单位（kg↔g、kg↔t、L↔ml）。无法换算返回 None。
fn convert_amount(quantity: f64, from: &str, to: &str) -> Option<f64> {
    let from = normalize_unit(from);
    let to = normalize_unit(to);
    if from == to {
        return Some(quantity);
    }
    // 未指定单位 → 假设与库存单位一致；库存未定义单位 → 接受
    if from.is_empty() || to.is_empty() {
        return Some(quantity);
    }
    let factor = match (from.as_str(), to.as_str()) {
        ("kg", "g") => 1000.0,
        ("g", "kg") => 0.001,
        ("t", "kg") => 1000.0,
        ("kg", "t") => 0.001,
        ("L", "ml") => 1000.0,
        ("ml", "L") => 0.001,
        _ => return None,
    };
    Some(quantity * factor)
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
    let Some(items) = details.get("items").and_then(|i| i.as_array()) else {
        return Vec::new();
    };
    let mut out: Vec<UsageItem> = Vec::new();
    match category {
        "施肥" => {
            for it in items {
                let name = it.get("name").and_then(|n| n.as_str()).unwrap_or("").trim().to_string();
                let amount = it
                    .get("amount")
                    .and_then(|a| a.as_str())
                    .and_then(parse_amount)
                    .or_else(|| it.get("amount").and_then(|a| a.as_f64()).map(|v| (v, String::new())))
                    .unwrap_or((0.0, String::new()));
                if !name.is_empty() && amount.0 > 0.0 {
                    out.push(UsageItem { name, amount: amount.0, unit: amount.1, alt_names: Vec::new() });
                }
            }
        }
        "打药" => {
            for it in items {
                let brand = it.get("brand").and_then(|b| b.as_str()).unwrap_or("").trim().to_string();
                let ingredient = it.get("ingredient").and_then(|b| b.as_str()).unwrap_or("").trim().to_string();
                let name = if !brand.is_empty() { brand.clone() } else { ingredient.clone() };
                let amount = it.get("usage").and_then(|a| a.as_f64()).unwrap_or(0.0);
                let unit = it.get("usage_unit").and_then(|u| u.as_str()).unwrap_or("").trim().to_string();
                if !name.is_empty() && amount > 0.0 {
                    out.push(UsageItem {
                        name,
                        amount,
                        unit,
                        alt_names: vec![ingredient, brand],
                    });
                }
            }
        }
        _ => {}
    }
    out
}

async fn find_inventory(
    tx: &mut Transaction<'_, sqlx::Sqlite>,
    name: &str,
    alt_names: &[String],
) -> Result<Option<sqlx::sqlite::SqliteRow>, sqlx::Error> {
    let exact = sqlx::query(
        "SELECT id, name, unit, stock FROM inventory_items WHERE name = ? ORDER BY created_at DESC LIMIT 1",
    )
    .bind(name)
    .fetch_optional(&mut **tx)
    .await?;
    if exact.is_some() {
        return Ok(exact);
    }
    for alt in alt_names.iter().filter(|s| !s.trim().is_empty()) {
        let row = sqlx::query(
            "SELECT id, name, unit, stock FROM inventory_items WHERE name LIKE ? ORDER BY created_at DESC LIMIT 1",
        )
        .bind(format!("%{}%", alt.trim()))
        .fetch_optional(&mut **tx)
        .await?;
        if row.is_some() {
            return Ok(row);
        }
    }
    Ok(None)
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
    let mut warnings: Vec<String> = Vec::new();
    let items = parse_usage_items(category, details);
    for item in items {
        if item.name.trim().is_empty() || item.amount <= 0.0 {
            continue;
        }
        let Some(inv) = find_inventory(tx, &item.name, &item.alt_names).await? else {
            warnings.push(format!("库存中未找到「{}」，未扣减", item.name));
            continue;
        };
        let inv_id: String = match inv.try_get("id") {
            Ok(v) => v,
            Err(_) => continue,
        };
        let inv_unit: String = inv.try_get("unit").unwrap_or_default();
        let stock: f64 = inv.try_get("stock").unwrap_or(0.0);

        let Some(qty) = convert_amount(item.amount, &item.unit, &inv_unit) else {
            let left = if item.unit.is_empty() { inv_unit.clone() } else { item.unit.clone() };
            warnings.push(format!("「{}」用量单位 {} 与库存单位 {} 不匹配，未扣减", item.name, left, inv_unit));
            continue;
        };
        if qty <= 0.0 {
            continue;
        }
        let out_qty = if stock >= qty { qty } else {
            warnings.push(format!("「{}」库存不足（剩 {:.1}），已按实际出库", item.name, stock));
            stock
        };
        if out_qty <= 0.0 {
            continue;
        }
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO inventory_transactions (id, item_id, txn_type, quantity, operator, related_type, related_id, note, created_at) VALUES (?, ?, 'out', ?, ?, 'farm_operation', ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&inv_id)
        .bind(out_qty)
        .bind(if operator.is_empty() { "系统" } else { operator })
        .bind(op_id)
        .bind(format!("农事记录自动扣减（{}）", category))
        .bind(now)
        .execute(&mut **tx)
        .await?;
        sqlx::query("UPDATE inventory_items SET stock = stock - ?, updated_at = ? WHERE id = ?")
            .bind(out_qty)
            .bind(now)
            .bind(&inv_id)
            .execute(&mut **tx)
            .await?;
    }
    Ok(warnings)
}

/// 回滚某农事操作关联的出库流水：恢复库存并删除流水（须在事务内调用）
pub async fn revert_op_deductions(
    tx: &mut Transaction<'_, sqlx::Sqlite>,
    op_id: &str,
) -> Result<(), sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, item_id, quantity FROM inventory_transactions WHERE related_type = 'farm_operation' AND related_id = ?",
    )
    .bind(op_id)
    .fetch_all(&mut **tx)
    .await?;
    for row in rows {
        let txn_id: String = row.try_get("id")?;
        let item_id: String = row.try_get("item_id")?;
        let qty: f64 = row.try_get("quantity").unwrap_or(0.0);
        if qty > 0.0 {
            sqlx::query("UPDATE inventory_items SET stock = stock + ? WHERE id = ?")
                .bind(qty)
                .bind(&item_id)
                .execute(&mut **tx)
                .await?;
        }
        sqlx::query("DELETE FROM inventory_transactions WHERE id = ?")
            .bind(&txn_id)
            .execute(&mut **tx)
            .await?;
    }
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
        assert_eq!(parse_amount(" 2.5公斤 "), Some((2.5, "kg".to_string())));
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