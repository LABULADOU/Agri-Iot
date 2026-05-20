use crate::models::{VentType, VentilationController, VentilationDecision};

impl VentilationController {
    /// 估算执行所需时间
    pub fn estimate_duration(&self, target_percent: f64, vent_type: VentType) -> i64 {
        let current = match vent_type {
            VentType::Top => self.top_vent_current,
            VentType::Side => self.side_vent_current,
        };
        let diff = (target_percent - current).abs();
        (diff / 10.0 * 5.0) as i64
    }
}

/// 根据传感器数据计算通风建议
pub fn recommend_ventilation(
    controller: &VentilationController,
    air_temp: f64,
    air_humidity: f64,
    target_temp: f64,
    target_humidity: f64,
) -> VentilationDecision {
    controller.calculate_target_position(target_temp, air_temp, target_humidity, air_humidity, VentType::Top)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_controller() -> VentilationController {
        VentilationController {
            top_vent_range: (0.0, 100.0),
            side_vent_range: (0.0, 80.0),
            top_vent_current: 30.0,
            side_vent_current: 20.0,
        }
    }

    #[test]
    fn test_ventilation_high_temp() {
        let ctrl = make_controller();
        let decision = ctrl.calculate_target_position(25.0, 35.0, 65.0, 50.0, VentType::Top);
        assert!(decision.target_percent > 30.0);
        assert_eq!(decision.priority, crate::models::ActionPriority::High);
    }

    #[test]
    fn test_emergency_close() {
        let ctrl = make_controller();
        let action = ctrl.emergency_close(VentType::Top);
        assert!(action.is_emergency);
        assert!(!action.requires_confirmation);
        assert_eq!(action.target_percent, 0.0);
    }

    #[test]
    fn test_side_vent() {
        let ctrl = make_controller();
        let decision = ctrl.calculate_target_position(25.0, 35.0, 65.0, 80.0, VentType::Side);
        assert!(decision.target_percent > 0.0);
        assert!(decision.target_percent <= 80.0); // side_vent_range max = 80
    }

    #[test]
    fn test_humidity_driven() {
        let ctrl = make_controller();
        // 温度正常但湿度极高 → 通风应以除湿为主
        let decision = ctrl.calculate_target_position(25.0, 24.0, 65.0, 95.0, VentType::Top);
        assert!(decision.target_percent > 20.0);
    }

    #[test]
    fn test_estimate_duration() {
        let ctrl = make_controller();
        // top_vent_current=30, target=80, diff=50, (50/10)*5 = 25 min
        let dur = ctrl.estimate_duration(80.0, VentType::Top);
        assert_eq!(dur, 25);
        // same position → 0
        let dur2 = ctrl.estimate_duration(30.0, VentType::Top);
        assert_eq!(dur2, 0);
    }

    #[test]
    fn test_recommend_ventilation() {
        let ctrl = make_controller();
        let decision = recommend_ventilation(&ctrl, 35.0, 50.0, 25.0, 65.0);
        assert!(decision.target_percent > 0.0);
    }
}
