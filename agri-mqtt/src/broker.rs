use anyhow::Result;
use rumqttd::{Broker, Config, ConnectionSettings, ServerSettings};
use std::collections::HashMap;
use std::net::SocketAddr;
use tracing::info;

pub fn start_broker(port: u16) -> Result<()> {
    let ws_port = port + 1;
    let console_port = port + 2;
    let config = Config {
        id: 0,
        router: rumqttd::RouterConfig {
            max_connections: 1000,
            max_outgoing_packet_count: 1_000_000,
            max_segment_size: 1_000_000,
            max_segment_count: 10_000,
            ..Default::default()
        },
        v4: {
            let mut servers = HashMap::new();
            servers.insert(
                "tcp".to_string(),
                ServerSettings {
                    name: "tcp".to_string(),
                    listen: SocketAddr::from(([127, 0, 0, 1], port)),
                    tls: None,
                    next_connection_delay_ms: 1,
                    connections: ConnectionSettings {
                        connection_timeout_ms: 60_000,
                        max_payload_size: 268_435_456,
                        max_inflight_count: 5000,
                        auth: None,
                        dynamic_filters: false,
                    },
                },
            );
            servers
        },
        ws: Some({
            let mut ws_servers = HashMap::new();
            ws_servers.insert(
                "ws".to_string(),
                ServerSettings {
                    name: "ws".to_string(),
                    listen: SocketAddr::from(([127, 0, 0, 1], ws_port)),
                    tls: None,
                    next_connection_delay_ms: 1,
                    connections: ConnectionSettings {
                        connection_timeout_ms: 60_000,
                        max_payload_size: 268_435_456,
                        max_inflight_count: 5000,
                        auth: None,
                        dynamic_filters: false,
                    },
                },
            );
            ws_servers
        }),
        console: {
            let mut cs = rumqttd::ConsoleSettings::default();
            cs.listen = format!("127.0.0.1:{}", console_port);
            cs
        },
        ..Default::default()
    };

    info!("MQTT Broker starting — TCP:{}, WS:{}", port, ws_port);

    let mut broker = Broker::new(config);
    broker.start()?;

    Ok(())
}
