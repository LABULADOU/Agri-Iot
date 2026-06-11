use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq)]
pub enum RainState {
    Dry,
    Detecting { since: Instant },
    Confirmed { since: Instant, intensity: RainIntensity },
    Monitoring { since: Instant, intensity: RainIntensity },
    Recovering { since: Instant },
}

#[derive(Debug, Clone, PartialEq)]
pub enum RainIntensity {
    None,
    Drizzle,
    Light,
    Moderate,
    Heavy,
    Storm,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WindState {
    Normal,
    GustDetected { since: Instant },
    Confirmed { since: Instant },
}

#[derive(Debug, Clone)]
pub struct DeviceState {
    pub rain: RainState,
    pub wind: WindState,
    pub since: Instant,
    pub hourly_shower_count: u8,
    pub vent_position: f64,
}

impl DeviceState {
    pub fn new() -> Self {
        Self {
            rain: RainState::Dry,
            wind: WindState::Normal,
            since: Instant::now(),
            hourly_shower_count: 0,
            vent_position: 0.0,
        }
    }
}

pub struct StateRegistry {
    states: HashMap<String, DeviceState>,
}

impl StateRegistry {
    pub fn new() -> Self {
        Self { states: HashMap::new() }
    }

    pub fn get(&self, node_id: &str) -> Option<&DeviceState> {
        self.states.get(node_id)
    }

    pub fn get_mut(&mut self, node_id: &str) -> &mut DeviceState {
        self.states.entry(node_id.to_string()).or_insert_with(DeviceState::new)
    }

    pub fn all_states(&self) -> &HashMap<String, DeviceState> {
        &self.states
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_state_registry() {
        let mut reg = StateRegistry::new();
        let state = reg.get_mut("node-001");
        assert_eq!(state.vent_position, 0.0);
        state.vent_position = 50.0;
        assert_eq!(reg.get("node-001").unwrap().vent_position, 50.0);
    }
}
