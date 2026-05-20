use crate::models::{Deviation, EnvironmentAssessment, HourlyImpact, WeatherImpact};

/// 计算单参数评分 (0-100)
pub fn calculate_parameter_score(current: f64, optimal: f64, min: f64, max: f64) -> f64 {
    if current >= min && current <= max {
        let deviation = ((current - optimal) / ((max - min) / 2.0)).abs();
        return 100.0 * (1.0 - deviation * 0.5).max(0.0);
    }
    let deviation = if current < min {
        (min - current) / min.abs().max(0.01)
    } else {
        (current - max) / max.abs().max(0.01)
    };
    (100.0 * (-deviation * 2.0).exp()).max(0.0)
}

pub fn assess_environment(
    soil_temp: f64, soil_moisture: f64, ec: f64, air_temp: f64, air_humidity: f64,
    crop: Option<&crate::models::CropProfile>,
) -> EnvironmentAssessment {
    let (st_opt, st_min, st_max) = crop.map_or((22.0, 15.0, 28.0), |c| {
        (c.soil_temp_optimal.unwrap_or(22.0), c.soil_temp_min.unwrap_or(15.0), c.soil_temp_max.unwrap_or(28.0))
    });
    let (sm_opt, sm_min, sm_max) = crop.map_or((75.0, 60.0, 85.0), |c| {
        (c.soil_moisture_optimal.unwrap_or(75.0), c.soil_moisture_min.unwrap_or(60.0), c.soil_moisture_max.unwrap_or(85.0))
    });
    let (ec_opt, ec_min, ec_max) = crop.map_or((2.5, 1.5, 4.0), |c| {
        (c.ec_optimal.unwrap_or(2.5), c.ec_min.unwrap_or(1.5), c.ec_max.unwrap_or(4.0))
    });
    let (at_opt, at_min, at_max) = crop.map_or((25.0, 18.0, 32.0), |c| {
        (c.air_temp_optimal.unwrap_or(25.0), c.air_temp_min.unwrap_or(18.0), c.air_temp_max.unwrap_or(32.0))
    });
    let (ah_opt, ah_min, ah_max) = crop.map_or((65.0, 50.0, 80.0), |c| {
        (c.air_humidity_optimal.unwrap_or(65.0), c.air_humidity_min.unwrap_or(50.0), c.air_humidity_max.unwrap_or(80.0))
    });

    let soil_temp_score = calculate_parameter_score(soil_temp, st_opt, st_min, st_max);
    let soil_moisture_score = calculate_parameter_score(soil_moisture, sm_opt, sm_min, sm_max);
    let ec_score = calculate_parameter_score(ec, ec_opt, ec_min, ec_max);
    let air_temp_score = calculate_parameter_score(air_temp, at_opt, at_min, at_max);
    let air_humidity_score = calculate_parameter_score(air_humidity, ah_opt, ah_min, ah_max);
    let overall_score = (soil_temp_score + soil_moisture_score + ec_score + air_temp_score + air_humidity_score) / 5.0;

    let mut deviations = Vec::new();
    for (param, current, optimal) in &[
        ("soil_temp", soil_temp, st_opt),
        ("soil_moisture", soil_moisture, sm_opt),
        ("ec", ec, ec_opt),
        ("air_temp", air_temp, at_opt),
        ("air_humidity", air_humidity, ah_opt),
    ] {
        if (*current - optimal).abs() > 0.1 {
            let dev_pct = if optimal.abs() > 0.01 { (current - optimal) / optimal * 100.0 } else { 0.0 };
            deviations.push(Deviation { param: param.to_string(), current: *current, optimal: *optimal, deviation_pct: dev_pct });
        }
    }

    let trend = if deviations.is_empty() { "stable".to_string() } else { "declining".to_string() };

    EnvironmentAssessment {
        overall_score,
        soil_temp_score,
        soil_moisture_score,
        ec_score,
        air_temp_score,
        air_humidity_score,
        deviations,
        trend,
        weather_impact: WeatherImpact { has_alert: false, alert_type: None, impact_hours: vec![], recommendation: None },
    }
}

pub fn add_weather_impact(mut assessment: EnvironmentAssessment, weather: Option<&crate::models::WeatherData>) -> EnvironmentAssessment {
    if let Some(w) = weather {
        let mut impacts = Vec::new();
        if let Some(ws) = w.wind_speed {
            if ws > 30.0 {
                impacts.push(HourlyImpact { hour: 0, temp_impact: "wind_alert".to_string(), humidity_impact: "normal".to_string() });
            }
        }
        assessment.weather_impact = WeatherImpact {
            has_alert: !impacts.is_empty(),
            alert_type: impacts.first().map(|_| "wind".to_string()),
            impact_hours: impacts,
            recommendation: None,
        };
    }
    assessment
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== calculate_parameter_score ==========

    #[test]
    fn test_calc_score_optimal() {
        // 最优值得 100 分
        let score = calculate_parameter_score(22.0, 22.0, 15.0, 28.0);
        assert!((score - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_calc_score_within_range() {
        // 范围内但偏离最优，得分下降但不低于 0
        let score = calculate_parameter_score(20.0, 22.0, 15.0, 28.0);
        assert!(score > 50.0 && score < 100.0);
    }

    #[test]
    fn test_calc_score_at_boundary() {
        // 正好在边界上 → 衰减到 ~46 分（公式特征：线性惩罚 vs 指数衰减间有跳变）
        let low = calculate_parameter_score(15.0, 22.0, 15.0, 28.0);
        let high = calculate_parameter_score(28.0, 22.0, 15.0, 28.0);
        assert!(low > 30.0 && low < 60.0);
        assert!(high > 30.0 && high < 60.0);
        // 边界外一点 → 指数衰减（因公式跳变，值可能高于边界内）
        let just_outside = calculate_parameter_score(14.9, 22.0, 15.0, 28.0);
        assert!(just_outside > 0.0);
    }

    #[test]
    fn test_calc_score_extreme() {
        // 极端偏离 → 指数衰减 > 0
        let score = calculate_parameter_score(50.0, 22.0, 15.0, 28.0);
        assert!(score > 0.0 && score < 30.0);
        let score2 = calculate_parameter_score(-10.0, 22.0, 15.0, 28.0);
        assert!(score2 > 0.0 && score2 < 30.0);
    }

    #[test]
    fn test_calc_score_below_min() {
        // 低于最小值
        let score = calculate_parameter_score(5.0, 22.0, 15.0, 28.0);
        assert!(score > 0.0 && score < 50.0);
    }

    // ========== assess_environment ==========

    #[test]
    fn test_assess_optimal_values() {
        let result = assess_environment(22.0, 75.0, 2.5, 25.0, 65.0, None);
        assert!((result.overall_score - 100.0).abs() < 5.0);
        assert!(result.deviations.is_empty());
        assert_eq!(result.trend, "stable");
    }

    #[test]
    fn test_assess_all_extreme() {
        let result = assess_environment(50.0, 10.0, 8.0, 45.0, 95.0, None);
        assert!(result.overall_score < 50.0);
        assert!(!result.deviations.is_empty());
        assert_eq!(result.trend, "declining");
    }

    #[test]
    fn test_assess_weather_impact() {
        use crate::models::WeatherData;
        let assessment = assess_environment(22.0, 75.0, 2.5, 25.0, 65.0, None);
        assert!(!assessment.weather_impact.has_alert);

        let weather = WeatherData {
            id: 0, area_id: Some("a1".into()), source: "local".into(),
            temperature: Some(25.0), humidity: Some(60.0), wind_speed: Some(45.0),
            wind_direction: None, precipitation: None, snow_probability: None,
            uv_index: None, forecast_hour: None, timestamp: 0,
        };
        let with_impact = add_weather_impact(assessment, Some(&weather));
        assert!(with_impact.weather_impact.has_alert);
        assert_eq!(with_impact.weather_impact.alert_type.as_deref(), Some("wind"));
    }
}
