use anyhow::Result;
use rumqttd::{Broker, Config, ConnectionSettings, ServerSettings};
use std::collections::HashMap;
use std::net::SocketAddr;
use tracing::info;

pub fn start_broker(port: u16) -> Result<()> {
    let mut config = Config::default();
    config.id = 0;
    config.router.max_connections = 1000;
    config.router.max_outgoing_packet_count = 1000;
    config.router.max_segment_size = 100_000;
    config.router.max_segment_count = 10;

    let mut servers = HashMap::new();
    servers.insert(
        "tcp".to_string(),
        ServerSettings {
            name: "tcp".to_string(),
            listen: SocketAddr::from(([127, 0, 0, 1], port)),
            tls: None,
            next_connection_delay_ms: 1,
            connections: ConnectionSettings {
                connection_timeout_ms: 10,
                max_payload_size: 268_435_456,
                max_inflight_count: 200,
                auth: None,
                dynamic_filters: false,
            },
        },
    );
    config.v4 = servers;

    let ws_port = port + 1;
    let mut ws_servers = HashMap::new();
    ws_servers.insert(
        "ws".to_string(),
        ServerSettings {
            name: "ws".to_string(),
            listen: SocketAddr::from(([127, 0, 0, 1], ws_port)),
            tls: None,
            next_connection_delay_ms: 1,
            connections: ConnectionSettings {
                connection_timeout_ms: 10,
                max_payload_size: 268_435_456,
                max_inflight_count: 200,
                auth: None,
                dynamic_filters: false,
            },
        },
    );
    config.ws = Some(ws_servers);

    config.console.listen = format!("127.0.0.1:{}", port + 2);

    info!("MQTT Broker starting — TCP:{}, WS:{}", port, ws_port);

    let mut broker = Broker::new(config);
    broker.start()?;

    Ok(())
}
