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
            listen: SocketAddr::from(([0, 0, 0, 0], port)),
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
    config.console.listen = format!("127.0.0.1:{}", port + 1);

    info!("MQTT Broker starting on port {} (max_connections={})", port, config.router.max_connections);

    let mut broker = Broker::new(config);
    broker.start()?;

    Ok(())
}
