use crate::models::NightModeConfig;

pub fn is_night_time(config: &NightModeConfig, now: chrono::DateTime<chrono::Utc>) -> bool {
    config.is_night_time(now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

    #[test]
    fn test_night_time_cross_midnight() {
        let config = NightModeConfig {
            enabled: true,
            start_time: NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
            end_time: NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
            enhanced_monitoring: true,
            reduced_action_threshold: 0.7,
            night_contact_list: vec![],
        };
        assert!(config.is_night_time_naive(NaiveTime::from_hms_opt(22, 0, 0).unwrap()));
        assert!(!config.is_night_time_naive(NaiveTime::from_hms_opt(12, 0, 0).unwrap()));
        assert!(config.is_night_time_naive(NaiveTime::from_hms_opt(3, 0, 0).unwrap()));
        assert!(!config.is_night_time_naive(NaiveTime::from_hms_opt(8, 0, 0).unwrap()));
    }

    #[test]
    fn test_night_time_non_cross_midnight() {
        let config = NightModeConfig {
            enabled: true,
            start_time: NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
            end_time: NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
            enhanced_monitoring: true,
            reduced_action_threshold: 0.7,
            night_contact_list: vec![],
        };
        // 非跨午夜：night = [06:00, 18:00]
        assert!(!config.is_night_time_naive(NaiveTime::from_hms_opt(22, 0, 0).unwrap()));
        assert!(config.is_night_time_naive(NaiveTime::from_hms_opt(12, 0, 0).unwrap()));
        assert!(!config.is_night_time_naive(NaiveTime::from_hms_opt(3, 0, 0).unwrap()));
        assert!(config.is_night_time_naive(NaiveTime::from_hms_opt(8, 0, 0).unwrap()));
    }

    #[test]
    fn test_is_night_time_utc_wrapper() {
        let config = NightModeConfig {
            enabled: true,
            start_time: NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
            end_time: NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
            enhanced_monitoring: true,
            reduced_action_threshold: 0.7,
            night_contact_list: vec![],
        };
        // is_night_time(Utc) 委托 is_night_time_naive(Local time)
        // 测试调用不 panic 即可
        let _ = config.is_night_time(chrono::Utc::now());
    }
}
