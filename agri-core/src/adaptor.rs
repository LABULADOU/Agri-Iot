use serde::Deserialize;
use serde_json::Map;

#[derive(Debug, Clone)]
pub struct ParsedTelemetry {
    pub node_id: String,
    pub metrics: Map<String, serde_json::Value>,
    pub seq: Option<i64>,
    pub boot_id: Option<String>,
    pub captured_at: Option<i64>,
}

#[derive(Debug)]
pub struct ParsedCommand {
    pub command: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug)]
pub enum AdaptorError {
    Parse(String),
    UnsupportedFormat(String),
    MissingField(String),
}

impl std::fmt::Display for AdaptorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdaptorError::Parse(msg) => write!(f, "Parse error: {}", msg),
            AdaptorError::UnsupportedFormat(fmt) => write!(f, "Unsupported format: {}", fmt),
            AdaptorError::MissingField(field) => write!(f, "Missing field: {}", field),
        }
    }
}

impl std::error::Error for AdaptorError {}

#[derive(Debug, Clone, PartialEq)]
pub enum PayloadFormat {
    Json,
    Protobuf,
}

impl PayloadFormat {
    pub fn from_content_type(ct: &str) -> Option<Self> {
        match ct {
            "application/json" | "json" => Some(PayloadFormat::Json),
            "application/protobuf" | "protobuf" | "application/x-protobuf" => Some(PayloadFormat::Protobuf),
            _ => None,
        }
    }
}

pub trait PayloadAdaptor: Send + Sync {
    fn parse_telemetry(&self, node_id: &str, payload: &[u8]) -> Result<ParsedTelemetry, AdaptorError>;
    fn parse_gateway_telemetry(&self, gateway_id: &str, payload: &[u8]) -> Result<Vec<ParsedTelemetry>, AdaptorError>;
    fn parse_status(&self, node_id: &str, payload: &[u8]) -> Result<String, AdaptorError>;
    fn serialize_command(&self, command: &ParsedCommand) -> Result<Vec<u8>, AdaptorError>;
    fn format(&self) -> PayloadFormat;
}

#[derive(Deserialize)]
struct GwDeviceEntry {
    node_id: Option<String>,
    name: Option<String>,
    metrics: Map<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct GwPayload {
    devices: Vec<GwDeviceEntry>,
    seq: Option<i64>,
    boot_id: Option<String>,
}

pub struct JsonPayloadAdaptor;

impl JsonPayloadAdaptor {
    pub fn new() -> Self {
        JsonPayloadAdaptor
    }
}

impl PayloadAdaptor for JsonPayloadAdaptor {
    fn parse_telemetry(&self, node_id: &str, payload: &[u8]) -> Result<ParsedTelemetry, AdaptorError> {
        let data: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|e| AdaptorError::Parse(format!("invalid JSON: {}", e)))?;

        let obj = data.as_object()
            .ok_or_else(|| AdaptorError::Parse("payload must be a JSON object".to_string()))?;

        let seq = data.get("seq").and_then(|s| s.as_i64());
        let boot_id = data.get("boot_id").and_then(|s| s.as_str()).map(String::from);
        let captured_at = data.get("captured_at").and_then(|s| s.as_i64()).filter(|t| *t > 100000);

        let metrics = match data.get("metrics").and_then(|m| m.as_object()) {
            Some(m) => m.clone(),
            None => {
                let mut flat = Map::new();
                for (k, v) in obj {
                    if k != "seq" && k != "boot_id" && k != "node_id" {
                        flat.insert(k.clone(), v.clone());
                    }
                }
                flat
            }
        };

        Ok(ParsedTelemetry {
            node_id: node_id.to_string(),
            metrics,
            seq,
            boot_id,
            captured_at,
        })
    }

    fn parse_gateway_telemetry(&self, gateway_id: &str, payload: &[u8]) -> Result<Vec<ParsedTelemetry>, AdaptorError> {
        let gw: GwPayload = serde_json::from_slice(payload)
            .map_err(|e| AdaptorError::Parse(format!("invalid gateway JSON: {}", e)))?;

        if gw.devices.is_empty() {
            return Err(AdaptorError::MissingField("devices".to_string()));
        }

        let results: Vec<ParsedTelemetry> = gw.devices.into_iter().map(|d| {
            let node_id = d.node_id.unwrap_or_else(|| format!("{}/{}", gateway_id, d.name.unwrap_or_else(|| "unknown".to_string())));
            ParsedTelemetry {
                node_id,
                metrics: d.metrics,
                seq: gw.seq,
                boot_id: gw.boot_id.clone(),
                captured_at: None,
            }
        }).collect();

        Ok(results)
    }

    fn parse_status(&self, _node_id: &str, payload: &[u8]) -> Result<String, AdaptorError> {
        let s = std::str::from_utf8(payload)
            .map_err(|e| AdaptorError::Parse(format!("invalid UTF-8: {}", e)))?;
        let data: serde_json::Value = serde_json::from_str(s).unwrap_or(serde_json::Value::Null);
        let status = data.get("status").and_then(|s| s.as_str()).unwrap_or(s.trim());
        Ok(match status {
            "online" => "online",
            _ => "offline",
        }.to_string())
    }

    fn serialize_command(&self, command: &ParsedCommand) -> Result<Vec<u8>, AdaptorError> {
        let payload = serde_json::json!({
            "command": command.command,
            "params": command.params,
        });
        serde_json::to_vec(&payload)
            .map_err(|e| AdaptorError::Parse(format!("serialization error: {}", e)))
    }

    fn format(&self) -> PayloadFormat {
        PayloadFormat::Json
    }
}

pub struct ProtobufPayloadAdaptor;

impl ProtobufPayloadAdaptor {
    pub fn new() -> Self {
        ProtobufPayloadAdaptor
    }
}

impl PayloadAdaptor for ProtobufPayloadAdaptor {
    fn parse_telemetry(&self, _node_id: &str, _payload: &[u8]) -> Result<ParsedTelemetry, AdaptorError> {
        Err(AdaptorError::UnsupportedFormat("Protobuf not yet implemented".to_string()))
    }

    fn parse_gateway_telemetry(&self, _gateway_id: &str, _payload: &[u8]) -> Result<Vec<ParsedTelemetry>, AdaptorError> {
        Err(AdaptorError::UnsupportedFormat("Protobuf not yet implemented".to_string()))
    }

    fn parse_status(&self, _node_id: &str, _payload: &[u8]) -> Result<String, AdaptorError> {
        Err(AdaptorError::UnsupportedFormat("Protobuf not yet implemented".to_string()))
    }

    fn serialize_command(&self, _command: &ParsedCommand) -> Result<Vec<u8>, AdaptorError> {
        Err(AdaptorError::UnsupportedFormat("Protobuf not yet implemented".to_string()))
    }

    fn format(&self) -> PayloadFormat {
        PayloadFormat::Protobuf
    }
}

pub fn default_adaptor() -> Box<dyn PayloadAdaptor> {
    Box::new(JsonPayloadAdaptor::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_parse_telemetry_with_metrics() {
        let adaptor = JsonPayloadAdaptor::new();
        let payload = br#"{"metrics": {"temperature": 25.5, "humidity": 60}, "seq": 100}"#;
        let result = adaptor.parse_telemetry("node-001", payload).unwrap();
        assert_eq!(result.node_id, "node-001");
        assert_eq!(result.seq, Some(100));
        assert_eq!(result.metrics.get("temperature").and_then(|v| v.as_f64()), Some(25.5));
        assert_eq!(result.metrics.get("humidity").and_then(|v| v.as_f64()), Some(60.0));
    }

    #[test]
    fn test_json_parse_telemetry_flat() {
        let adaptor = JsonPayloadAdaptor::new();
        let payload = br#"{"temperature": 25.5, "humidity": 60, "seq": 200}"#;
        let result = adaptor.parse_telemetry("node-001", payload).unwrap();
        assert_eq!(result.seq, Some(200));
        assert_eq!(result.metrics.get("temperature").and_then(|v| v.as_f64()), Some(25.5));
        assert_eq!(result.metrics.get("humidity").and_then(|v| v.as_f64()), Some(60.0));
        assert!(result.metrics.get("seq").is_none());
    }

    #[test]
    fn test_json_parse_gateway() {
        let adaptor = JsonPayloadAdaptor::new();
        let payload = br#"{"devices": [
            {"node_id": "soil-1", "metrics": {"soil_temperature": 19.2}},
            {"node_id": "dht-1", "metrics": {"temperature": 20.8, "humidity": 78.5}}
        ], "seq": 50}"#;
        let results = adaptor.parse_gateway_telemetry("gw-001", payload).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].node_id, "soil-1");
        assert_eq!(results[0].seq, Some(50));
        assert_eq!(results[1].node_id, "dht-1");
        assert_eq!(results[1].metrics.get("temperature").and_then(|v| v.as_f64()), Some(20.8));
    }

    #[test]
    fn test_json_parse_status() {
        let adaptor = JsonPayloadAdaptor::new();
        let result = adaptor.parse_status("node-001", b"{\"status\": \"online\"}").unwrap();
        assert_eq!(result, "online");
    }

    #[test]
    fn test_json_parse_status_plaintext() {
        let adaptor = JsonPayloadAdaptor::new();
        let result = adaptor.parse_status("node-001", b"online").unwrap();
        assert_eq!(result, "online");
    }

    #[test]
    fn test_json_serialize_command() {
        let adaptor = JsonPayloadAdaptor::new();
        let cmd = ParsedCommand {
            command: "switch".to_string(),
            params: Some(serde_json::json!({"state": "on"})),
        };
        let bytes = adaptor.serialize_command(&cmd).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["command"], "switch");
        assert_eq!(parsed["params"]["state"], "on");
    }

    #[test]
    fn test_protobuf_returns_error() {
        let adaptor = ProtobufPayloadAdaptor::new();
        let result = adaptor.parse_telemetry("node-001", b"some bytes");
        assert!(result.is_err());
        assert!(matches!(result, Err(AdaptorError::UnsupportedFormat(_))));
    }

    #[test]
    fn test_payload_format_from_content_type() {
        assert_eq!(PayloadFormat::from_content_type("application/json"), Some(PayloadFormat::Json));
        assert_eq!(PayloadFormat::from_content_type("application/protobuf"), Some(PayloadFormat::Protobuf));
        assert_eq!(PayloadFormat::from_content_type("text/xml"), None);
    }
}
