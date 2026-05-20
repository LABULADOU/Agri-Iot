use crate::models::{Action, Emergency, EmergencyType, CompareOp};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::time::Instant;

/// 天气预警输入
#[derive(Debug, Clone)]
pub struct WeatherAlertInput {
    pub wind_speed_kmh: Option<f64>,
    pub precipitation_mm_per_hour: Option<f64>,
    pub temperature_celsius: Option<f64>,
    pub snow_probability: Option<f64>,
    pub humidity: Option<f64>,
}

/// 紧急检测输出（扩展信息）
#[derive(Debug, Clone)]
pub struct EmergencyOutput {
    pub emergencies: Vec<Emergency>,
    pub pauses_auto_mode: bool,
}

/// 紧急检测上下文（追踪持续触发时间）
#[derive(Debug)]
pub struct EmergencyContext {
    /// 每个区域每种紧急类型的首次触发时间
    onset_times: HashMap<(String, EmergencyType), Instant>,
    /// 系统故障检测：上次检测时各设备的更新时间
    device_seen_at: HashMap<String, DateTime<Utc>>,
    /// 系统故障告警去重：设备最后触发 SystemFailure 的时间
    system_failure_fired_at: HashMap<String, DateTime<Utc>>,
}

impl Default for EmergencyContext {
    fn default() -> Self {
        Self::new()
    }
}

impl EmergencyContext {
    pub fn new() -> Self {
        Self {
            onset_times: HashMap::new(),
            device_seen_at: HashMap::new(),
            system_failure_fired_at: HashMap::new(),
        }
    }

    /// 检查某紧急类型在给定区域是否已持续超过指定分钟数
    fn has_passed_duration(&mut self, area_id: &str, et: &EmergencyType, duration_minutes: u32) -> bool {
        let key = (area_id.to_string(), et.clone());
        let now = Instant::now();

        match self.onset_times.get(&key) {
            Some(start) => {
                let elapsed = now.duration_since(*start);
                elapsed.as_secs() >= (duration_minutes as u64 * 60)
            }
            None => {
                self.onset_times.insert(key, now);
                false
            }
        }
    }

    /// 清除不再触发的紧急类型的 onset 记录
    pub fn clear_untracked(&mut self, area_id: &str, active_types: &[EmergencyType]) {
        self.onset_times.retain(|(aid, et), _| {
            aid != area_id || active_types.contains(et)
        });
    }

    /// 更新设备在线时间快照（用于 SystemFailure 检测）
    pub fn track_device(&mut self, device_id: &str, updated_at: DateTime<Utc>) {
        self.device_seen_at.insert(device_id.to_string(), updated_at);
    }
}

/// 检查紧急情况（基础版，无持续时间追踪）
/// 适用于一次性 API 调用
pub fn check_emergency_basic(weather: &WeatherAlertInput) -> Vec<Emergency> {
    let mut emergencies = Vec::new();

    // Rule 1: 大风保护 — 风速 > 40km/h
    if let Some(ws) = weather.wind_speed_kmh {
        if CompareOp::Gt.evaluate(ws, 40.0) {
            emergencies.push(Emergency {
                emergency_type: EmergencyType::StrongWind,
                confidence: 0.95,
                message: format!("检测到大风({:.1}km/h)，已自动关闭顶部通风口", ws),
                triggered_at: Utc::now(),
                pauses_auto_mode: false,
                night_additional_contact: false,
            });
        }
    }

    // Rule 2: 大雨保护 — 降水量 > 10mm/h
    if let Some(pr) = weather.precipitation_mm_per_hour {
        if CompareOp::Gt.evaluate(pr, 10.0) {
            emergencies.push(Emergency {
                emergency_type: EmergencyType::HeavyRain,
                confidence: 0.95,
                message: format!("检测到大雨({:.1}mm/h)，已自动关闭顶部通风口", pr),
                triggered_at: Utc::now(),
                pauses_auto_mode: false,
                night_additional_contact: false,
            });
        }
    }

    // Rule 3: 降雪保护 — 温度 < 3°C 且降雪概率 > 0.6
    if let (Some(temp), Some(snow_prob)) = (weather.temperature_celsius, weather.snow_probability) {
        if CompareOp::Lt.evaluate(temp, 3.0) && CompareOp::Gt.evaluate(snow_prob, 0.6) {
            emergencies.push(Emergency {
                emergency_type: EmergencyType::Snow,
                confidence: 0.85,
                message: format!("⚠️ 降雪风险预警！温度{:.1}°C，降雪概率{:.0}%。已关闭所有通风口，请立即前往现场！", temp, snow_prob * 100.0),
                triggered_at: Utc::now(),
                pauses_auto_mode: true,
                night_additional_contact: true,
            });
        }
    }

    // Rule 4: 极端高温 — 温度 > 38°C
    if let Some(temp) = weather.temperature_celsius {
        if CompareOp::Gt.evaluate(temp, 38.0) {
            emergencies.push(Emergency {
                emergency_type: EmergencyType::ExtremeHeat,
                confidence: 0.9,
                message: format!("极端高温警告，温度已达{:.1}°C，正在执行紧急通风", temp),
                triggered_at: Utc::now(),
                pauses_auto_mode: false,
                night_additional_contact: false,
            });
        }
    }

    // Rule 5: 极端低温 — 温度 < 5°C
    if let Some(temp) = weather.temperature_celsius {
        if CompareOp::Lt.evaluate(temp, 5.0) {
            emergencies.push(Emergency {
                emergency_type: EmergencyType::ExtremeCold,
                confidence: 0.9,
                message: format!("极端低温警告，温度{:.1}°C，正在关闭通风口", temp),
                triggered_at: Utc::now(),
                pauses_auto_mode: false,
                night_additional_contact: false,
            });
        }
    }

    emergencies
}

/// 检查紧急情况（带持续时间追踪）
/// 适用于规则引擎循环调用
pub fn check_emergency(
    weather: &WeatherAlertInput,
    ctx: &mut EmergencyContext,
    area_id: &str,
) -> EmergencyOutput {
    let mut emergencies = Vec::new();
    let mut pauses_auto_mode = false;

    // 获取基础检测结果
    let basic = check_emergency_basic(weather);

    // 对需要持续时间验证的类型做二次检查
    for e in basic {
        let passes_duration = match e.emergency_type {
            // 立即触发，无需持续
            EmergencyType::StrongWind | EmergencyType::HeavyRain => true,
            // 降雪：立即触发
            EmergencyType::Snow => {
                pauses_auto_mode = true;
                true
            }
            // 极端高温：持续 10 分钟才触发
            EmergencyType::ExtremeHeat => {
                ctx.has_passed_duration(area_id, &EmergencyType::ExtremeHeat, 10)
            }
            // 极端低温：持续 15 分钟才触发
            EmergencyType::ExtremeCold => {
                ctx.has_passed_duration(area_id, &EmergencyType::ExtremeCold, 15)
            }
            // 系统故障需要额外判断
            EmergencyType::SystemFailure => false,
        };

        if passes_duration {
            emergencies.push(e);
        }
    }

    // SystemFailure：检查设备是否长时间未更新（带去重，冷却期1小时）
    let now = Utc::now();
    for (device_id, last_seen) in &ctx.device_seen_at {
        let elapsed_minutes = (now - *last_seen).num_minutes();
        if elapsed_minutes > 30 {
            let already_fired = ctx.system_failure_fired_at.get(device_id)
                .map(|last| (now - *last).num_minutes() < 60)
                .unwrap_or(false);
            if !already_fired {
                ctx.system_failure_fired_at.insert(device_id.clone(), now);
                emergencies.push(Emergency {
                    emergency_type: EmergencyType::SystemFailure,
                    confidence: 0.7,
                    message: format!("系统故障：设备 {} 超过 {} 分钟无数据", device_id, elapsed_minutes),
                    triggered_at: Utc::now(),
                    pauses_auto_mode: false,
                    night_additional_contact: false,
                });
            }
        }
    }

    // 清理不再活跃的 onset 记录
    let active_types: Vec<EmergencyType> = emergencies.iter().map(|e| e.emergency_type.clone()).collect();
    ctx.clear_untracked(area_id, &active_types);

    EmergencyOutput {
        emergencies,
        pauses_auto_mode,
    }
}

/// 根据紧急事件获取对应动作
pub fn get_emergency_action(emergency: &Emergency) -> Action {
    match emergency.emergency_type {
        EmergencyType::StrongWind | EmergencyType::HeavyRain | EmergencyType::Snow => Action {
            command: "CLOSE".to_string(),
            device_type: "top_vent".to_string(),
            target_percent: 0.0,
            requires_confirmation: false,
            is_emergency: true,
            notification: Some(emergency.message.clone()),
        },
        EmergencyType::ExtremeHeat => Action {
            command: "OPEN".to_string(),
            device_type: "vent".to_string(),
            target_percent: 100.0,
            requires_confirmation: true,
            is_emergency: true,
            notification: Some(emergency.message.clone()),
        },
        EmergencyType::ExtremeCold => Action {
            command: "CLOSE".to_string(),
            device_type: "vent".to_string(),
            target_percent: 0.0,
            requires_confirmation: true,
            is_emergency: true,
            notification: Some(emergency.message.clone()),
        },
        EmergencyType::SystemFailure => Action {
            command: "HALT".to_string(),
            device_type: "system".to_string(),
            target_percent: 0.0,
            requires_confirmation: true,
            is_emergency: true,
            notification: Some(emergency.message.clone()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strong_wind_detection() {
        let input = WeatherAlertInput {
            wind_speed_kmh: Some(45.0),
            precipitation_mm_per_hour: None,
            temperature_celsius: None,
            snow_probability: None,
            humidity: None,
        };
        let emergencies = check_emergency_basic(&input);
        assert_eq!(emergencies.len(), 1);
        assert_eq!(emergencies[0].emergency_type, EmergencyType::StrongWind);
        assert!(!emergencies[0].pauses_auto_mode);
        assert!(!emergencies[0].night_additional_contact);
    }

    #[test]
    fn test_no_emergency_normal_conditions() {
        let input = WeatherAlertInput {
            wind_speed_kmh: Some(10.0),
            precipitation_mm_per_hour: Some(1.0),
            temperature_celsius: Some(25.0),
            snow_probability: Some(0.0),
            humidity: Some(60.0),
        };
        assert!(check_emergency_basic(&input).is_empty());
    }

    #[test]
    fn test_snow_detection() {
        let input = WeatherAlertInput {
            wind_speed_kmh: None,
            precipitation_mm_per_hour: None,
            temperature_celsius: Some(1.0),
            snow_probability: Some(0.8),
            humidity: None,
        };
        let emergencies = check_emergency_basic(&input);
        assert!(emergencies.iter().any(|e| e.emergency_type == EmergencyType::Snow));
        // Snow 必须 pauses_auto_mode 和 night_additional_contact
        let snow = emergencies.iter().find(|e| e.emergency_type == EmergencyType::Snow).unwrap();
        assert!(snow.pauses_auto_mode);
        assert!(snow.night_additional_contact);
    }

    #[test]
    fn test_wind_boundary() {
        let below = WeatherAlertInput { wind_speed_kmh: Some(40.0), ..Default::default() };
        let above = WeatherAlertInput { wind_speed_kmh: Some(40.1), ..Default::default() };
        assert!(check_emergency_basic(&below).is_empty());
        assert_eq!(check_emergency_basic(&above).len(), 1);
    }

    #[test]
    fn test_extreme_heat() {
        let input = WeatherAlertInput {
            wind_speed_kmh: None, precipitation_mm_per_hour: None,
            temperature_celsius: Some(38.1), snow_probability: None, humidity: None,
        };
        let ems = check_emergency_basic(&input);
        assert!(ems.iter().any(|e| e.emergency_type == EmergencyType::ExtremeHeat));
    }

    #[test]
    fn test_extreme_cold_no_snow() {
        let input = WeatherAlertInput {
            wind_speed_kmh: None, precipitation_mm_per_hour: None,
            temperature_celsius: Some(4.0), snow_probability: Some(0.0), humidity: None,
        };
        let ems = check_emergency_basic(&input);
        assert!(ems.iter().any(|e| e.emergency_type == EmergencyType::ExtremeCold));
        assert!(!ems.iter().any(|e| e.emergency_type == EmergencyType::Snow));
    }

    #[test]
    fn test_rain_detection() {
        let below = WeatherAlertInput { precipitation_mm_per_hour: Some(10.0), ..Default::default() };
        let above = WeatherAlertInput { precipitation_mm_per_hour: Some(10.1), ..Default::default() };
        assert!(check_emergency_basic(&below).is_empty());
        assert!(check_emergency_basic(&above).iter().any(|e| e.emergency_type == EmergencyType::HeavyRain));
    }

    #[test]
    fn test_get_emergency_action_all() {
        use crate::models::EmergencyType as ET;
        for et in &[ET::StrongWind, ET::HeavyRain, ET::Snow, ET::ExtremeHeat, ET::ExtremeCold, ET::SystemFailure] {
            let e = Emergency {
                emergency_type: et.clone(),
                confidence: 1.0,
                message: "test".into(),
                triggered_at: chrono::Utc::now(),
                pauses_auto_mode: false,
                night_additional_contact: false,
            };
            let action = get_emergency_action(&e);
            assert!(!action.command.is_empty());
            assert!(action.is_emergency);
        }
    }

    #[test]
    fn test_duration_tracking_heat_not_immediate() {
        let mut ctx = EmergencyContext::new();
        let input = WeatherAlertInput {
            wind_speed_kmh: None, precipitation_mm_per_hour: None,
            temperature_celsius: Some(39.0), snow_probability: None, humidity: None,
        };

        // 首次调用：尚未持续10分钟，不应触发
        let output = check_emergency(&input, &mut ctx, "area-1");
        assert!(!output.emergencies.iter().any(|e| e.emergency_type == EmergencyType::ExtremeHeat));
        assert!(!output.pauses_auto_mode);
    }

    #[test]
    fn test_duration_tracking_cold_not_immediate() {
        let mut ctx = EmergencyContext::new();
        let input = WeatherAlertInput {
            wind_speed_kmh: None, precipitation_mm_per_hour: None,
            temperature_celsius: Some(3.0), snow_probability: None, humidity: None,
        };

        // 首次调用：尚未持续15分钟，不应触发 ExtremeCold
        let output = check_emergency(&input, &mut ctx, "area-1");
        assert!(!output.emergencies.iter().any(|e| e.emergency_type == EmergencyType::ExtremeCold));
    }

    #[test]
    fn test_snow_pauses_auto_mode() {
        let mut ctx = EmergencyContext::new();
        let input = WeatherAlertInput {
            wind_speed_kmh: None, precipitation_mm_per_hour: None,
            temperature_celsius: Some(1.0), snow_probability: Some(0.8), humidity: None,
        };

        let output = check_emergency(&input, &mut ctx, "area-1");
        assert!(output.emergencies.iter().any(|e| e.emergency_type == EmergencyType::Snow));
        assert!(output.pauses_auto_mode);
    }

    #[test]
    fn test_system_failure_detection() {
        let mut ctx = EmergencyContext::new();
        let old_time = Utc::now() - chrono::Duration::minutes(45);
        ctx.track_device("dev-offline", old_time);

        let input = WeatherAlertInput {
            wind_speed_kmh: None, precipitation_mm_per_hour: None,
            temperature_celsius: Some(25.0), snow_probability: None, humidity: None,
        };

        let output = check_emergency(&input, &mut ctx, "area-1");
        assert!(output.emergencies.iter().any(|e| e.emergency_type == EmergencyType::SystemFailure));
    }

    #[test]
    fn test_system_failure_dedup() {
        let mut ctx = EmergencyContext::new();
        let old_time = Utc::now() - chrono::Duration::minutes(45);
        ctx.track_device("dev-offline", old_time);

        let input = WeatherAlertInput {
            wind_speed_kmh: None, precipitation_mm_per_hour: None,
            temperature_celsius: Some(25.0), snow_probability: None, humidity: None,
        };

        // 第一次：应触发
        let output1 = check_emergency(&input, &mut ctx, "area-1");
        assert!(output1.emergencies.iter().any(|e| e.emergency_type == EmergencyType::SystemFailure));

        // 第二次（同一设备，仍在离线）：冷却期60分钟，不应重复触发
        let output2 = check_emergency(&input, &mut ctx, "area-1");
        assert!(!output2.emergencies.iter().any(|e| e.emergency_type == EmergencyType::SystemFailure));
    }

    #[test]
    fn test_no_system_failure_recent() {
        let mut ctx = EmergencyContext::new();
        let recent = Utc::now() - chrono::Duration::minutes(5);
        ctx.track_device("dev-online", recent);

        let input = WeatherAlertInput {
            wind_speed_kmh: None, precipitation_mm_per_hour: None,
            temperature_celsius: Some(25.0), snow_probability: None, humidity: None,
        };

        let output = check_emergency(&input, &mut ctx, "area-1");
        assert!(!output.emergencies.iter().any(|e| e.emergency_type == EmergencyType::SystemFailure));
    }

    #[test]
    fn test_wind_immediate_no_duration() {
        let mut ctx = EmergencyContext::new();
        let input = WeatherAlertInput {
            wind_speed_kmh: Some(50.0), ..Default::default()
        };

        // 大风立即触发，无需等待
        let output = check_emergency(&input, &mut ctx, "area-1");
        assert!(output.emergencies.iter().any(|e| e.emergency_type == EmergencyType::StrongWind));
    }

    #[test]
    fn test_clear_untracked() {
        let mut ctx = EmergencyContext::new();
        // 模拟一次检测触发 ExtremeHeat 追踪
        let _ = ctx.has_passed_duration("area-1", &EmergencyType::ExtremeHeat, 10);
        assert!(ctx.onset_times.len() == 1);

        // 清理（只保留 Snow）
        ctx.clear_untracked("area-1", &[EmergencyType::Snow]);
        assert_eq!(ctx.onset_times.len(), 0);
    }

    #[test]
    fn test_compare_op_evaluate() {
        assert!(CompareOp::Gt.evaluate(5.0, 3.0));
        assert!(!CompareOp::Gt.evaluate(2.0, 3.0));
        assert!(CompareOp::Lt.evaluate(1.0, 3.0));
        assert!(!CompareOp::Lt.evaluate(5.0, 3.0));
        assert!(CompareOp::Eq.evaluate(3.0, 3.0005));
        assert!(!CompareOp::Eq.evaluate(3.0, 3.5));
    }
}

impl Default for WeatherAlertInput {
    fn default() -> Self {
        Self {
            wind_speed_kmh: None, precipitation_mm_per_hour: None,
            temperature_celsius: None, snow_probability: None, humidity: None,
        }
    }
}
