use anyhow::Result;
use rumqttd::{Broker, Config, ConnectionSettings, ServerSettings};
use std::collections::HashMap;
use std::net::SocketAddr;
use tracing::info;

pub fn create_broker_config(port: u16) -> Config {
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

    Config {
        id: 0,
        v4: servers,
        ..Default::default()
    }
}

pub fn start_broker(port: u16) -> Result<()> {
    let config = create_broker_config(port);
    let mut broker = Broker::new(config);

    info!("MQTT Broker starting on port {}", port);

    broker.start()?;

    Ok(())
}
