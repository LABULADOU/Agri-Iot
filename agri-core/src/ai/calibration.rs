use crate::models::CalibrationResult;

/// 校准结果模拟（实际实现需与设备通信）
pub fn calibrate_ventilator(device_id: &str, _area_id: &str) -> CalibrationResult {
    CalibrationResult {
        device_id: device_id.to_string(),
        range: (0.0, 100.0),
        calibration_date: chrono::Utc::now(),
        verified: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calibration_result() {
        let result = calibrate_ventilator("vent-001", "area-1");
        assert_eq!(result.device_id, "vent-001");
        assert!(result.verified);
        assert_eq!(result.range, (0.0, 100.0));
    }
}
