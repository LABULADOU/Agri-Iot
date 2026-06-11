pub const TELEMETRY: &str = "agri/node/{node_id}/telemetry";
pub const STATUS: &str = "agri/node/{node_id}/status";
pub const COMMAND_PREFIX: &str = "agri/node/{node_id}/command";
pub const GATEWAY_TELEMETRY: &str = "agri/gateway/{gateway_id}/telemetry";
pub const GATEWAY_STATUS: &str = "agri/gateway/{gateway_id}/status";

pub const PREFIX_NODE: &str = "agri/node/";
pub const PREFIX_GATEWAY: &str = "agri/gateway/";

pub enum TopicMatch {
    Telemetry { node_id: String },
    Status { node_id: String },
    Command { node_id: String },
    GatewayTelemetry { gateway_id: String },
    GatewayStatus { gateway_id: String },
}

pub fn match_topic(topic: &str) -> Option<TopicMatch> {
    if let Some(rest) = topic.strip_prefix(PREFIX_NODE) {
        let parts: Vec<&str> = rest.splitn(2, '/').collect();
        if parts.len() == 2 {
            let node_id = parts[0].to_string();
            match parts[1] {
                "telemetry" => Some(TopicMatch::Telemetry { node_id }),
                "status" => Some(TopicMatch::Status { node_id }),
                _ if parts[1].starts_with("command") => Some(TopicMatch::Command { node_id }),
                _ => None,
            }
        } else {
            None
        }
    } else if let Some(rest) = topic.strip_prefix(PREFIX_GATEWAY) {
        let parts: Vec<&str> = rest.splitn(2, '/').collect();
        if parts.len() == 2 {
            let gateway_id = parts[0].to_string();
            match parts[1] {
                "telemetry" => Some(TopicMatch::GatewayTelemetry { gateway_id }),
                "status" => Some(TopicMatch::GatewayStatus { gateway_id }),
                _ => None,
            }
        } else {
            None
        }
    } else {
        None
    }
}

pub fn telemetry_topic(node_id: &str) -> String {
    format!("agri/node/{}/telemetry", node_id)
}

pub fn status_topic(node_id: &str) -> String {
    format!("agri/node/{}/status", node_id)
}

pub fn command_topic(node_id: &str, cmd_id: &str) -> String {
    format!("agri/node/{}/command/{}", node_id, cmd_id)
}

pub fn gateway_telemetry_topic(gateway_id: &str) -> String {
    format!("agri/gateway/{}/telemetry", gateway_id)
}

pub fn gateway_status_topic(gateway_id: &str) -> String {
    format!("agri/gateway/{}/status", gateway_id)
}

pub fn subscribe_topics() -> Vec<String> {
    vec![
        "agri/node/+/telemetry".to_string(),
        "agri/node/+/status".to_string(),
        "agri/node/+/command/#".to_string(),
        "agri/gateway/+/telemetry".to_string(),
        "agri/gateway/+/status".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_telemetry() {
        let m = match_topic("agri/node/esp32-001/telemetry");
        assert!(matches!(m, Some(TopicMatch::Telemetry { node_id }) if node_id == "esp32-001"));
    }

    #[test]
    fn test_match_status() {
        let m = match_topic("agri/node/esp32-001/status");
        assert!(matches!(m, Some(TopicMatch::Status { node_id }) if node_id == "esp32-001"));
    }

    #[test]
    fn test_match_command() {
        let m = match_topic("agri/node/esp32-001/command/abc-123");
        assert!(matches!(m, Some(TopicMatch::Command { node_id }) if node_id == "esp32-001"));
    }

    #[test]
    fn test_match_gateway_telemetry() {
        let m = match_topic("agri/gateway/gw-001/telemetry");
        assert!(matches!(m, Some(TopicMatch::GatewayTelemetry { gateway_id }) if gateway_id == "gw-001"));
    }

    #[test]
    fn test_no_match() {
        assert!(match_topic("other/topic").is_none());
        assert!(match_topic("agri/node/only").is_none());
    }

    #[test]
    fn test_topic_formatting() {
        assert_eq!(telemetry_topic("n1"), "agri/node/n1/telemetry");
        assert_eq!(command_topic("n1", "cmd-1"), "agri/node/n1/command/cmd-1");
        assert_eq!(gateway_telemetry_topic("gw-1"), "agri/gateway/gw-1/telemetry");
    }
}
