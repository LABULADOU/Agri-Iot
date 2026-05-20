// EC 分析逻辑已在 models.rs 的 ECManager 中实现
// 此处仅提供测试和辅助函数

use crate::models::{ECManager, ECRecommendation, ECTrends};

pub fn analyze_ec(manager: &ECManager, current_ec: f64, trend: &ECTrends, area_id: &str) -> ECRecommendation {
    manager.analyze_ec(current_ec, trend, area_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ECTrend;
    use chrono::Utc;

    fn make_manager() -> ECManager {
        ECManager {
            optimal_ec_min: 1.5,
            optimal_ec_max: 4.0,
            warning_threshold_low: 0.5,
            warning_threshold_high: 6.0,
        }
    }

    #[test]
    fn test_ec_normal() {
        let manager = make_manager();
        let trend = ECTrends { readings: vec![], period_hours: 24 };
        let rec = manager.analyze_ec(2.5, &trend, "area-1");
        assert!(matches!(rec, ECRecommendation::NoAction));
    }

    #[test]
    fn test_ec_critical_low() {
        let manager = make_manager();
        let trend = ECTrends { readings: vec![], period_hours: 24 };
        let rec = manager.analyze_ec(0.3, &trend, "area-1");
        assert!(matches!(rec, ECRecommendation::ManualIntervention { .. }));
    }

    #[test]
    fn test_ec_high() {
        let manager = make_manager();
        let trend = ECTrends { readings: vec![], period_hours: 24 };
        let rec = manager.analyze_ec(5.0, &trend, "area-1");
        assert!(matches!(rec, ECRecommendation::DecreaseEC { .. }));
    }

    #[test]
    fn test_ec_increase() {
        let manager = make_manager();
        let trend = ECTrends { readings: vec![], period_hours: 24 };
        let rec = manager.analyze_ec(1.0, &trend, "area-1");
        assert!(matches!(rec, ECRecommendation::IncreaseEC { .. }));
    }

    // ========== ECTrends ==========

    #[test]
    fn test_ec_trend_insufficient() {
        let trend = ECTrends { readings: vec![(Utc::now(), 2.0)], period_hours: 24 };
        assert!(matches!(trend.analyze(), ECTrend::InsufficientData));
    }

    #[test]
    fn test_ec_trend_rising() {
        let now = Utc::now();
        let readings = vec![
            (now, 1.0),
            (now + chrono::Duration::hours(1), 2.0),
            (now + chrono::Duration::hours(2), 3.0),
        ];
        let trend = ECTrends { readings, period_hours: 24 };
        assert!(matches!(trend.analyze(), ECTrend::Rising));
    }

    #[test]
    fn test_ec_trend_falling() {
        let now = Utc::now();
        let readings = vec![
            (now, 3.0),
            (now + chrono::Duration::hours(1), 2.0),
            (now + chrono::Duration::hours(2), 1.0),
        ];
        let trend = ECTrends { readings, period_hours: 24 };
        assert!(matches!(trend.analyze(), ECTrend::Falling));
    }

    #[test]
    fn test_ec_trend_stable() {
        let now = Utc::now();
        let readings = vec![
            (now, 2.5),
            (now + chrono::Duration::hours(1), 2.52),
            (now + chrono::Duration::hours(2), 2.49),
        ];
        let trend = ECTrends { readings, period_hours: 24 };
        assert!(matches!(trend.analyze(), ECTrend::Stable));
    }

    #[test]
    fn test_analyze_ec_delegate() {
        let manager = make_manager();
        let trend = ECTrends { readings: vec![], period_hours: 24 };
        let rec = analyze_ec(&manager, 2.5, &trend, "area-1");
        assert!(matches!(rec, ECRecommendation::NoAction));
    }
}
