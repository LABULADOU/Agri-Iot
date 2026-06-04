use rumqttd::{Broker, Config, ConnectionSettings, ServerSettings};
use std::collections::HashMap;
use std::net::SocketAddr;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("agri_mqtt=info".parse().unwrap())
                .add_directive("rumqttd=info".parse().unwrap()),
        )
        .init();

    let port: u16 = std::env::var("MQTT_BROKER_PORT")
        .unwrap_or_else(|_| "1883".into())
        .parse()
        .unwrap_or(1883);

    let ws_port: u16 = std::env::var("MQTT_WS_PORT")
        .unwrap_or_else(|_| "1884".into())
        .parse()
        .unwrap_or(1884);

    let storage_path: String = std::env::var("MQTT_STORAGE_PATH")
        .unwrap_or_else(|_| "./mqtt-data".into());

    let bind_ip: std::net::IpAddr = std::env::var("MQTT_BIND_IP")
        .unwrap_or_else(|_| "0.0.0.0".into())  // LAN 访问需要 0.0.0.0
        .parse()
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));

    let mut config = Config::default();
    config.id = 0;
    config.router.max_connections = 1000;
    config.router.max_outgoing_packet_count = 100_000;
    config.router.max_segment_size = 1_000_000;
    config.router.max_segment_count = 1000;

    let mut v4 = HashMap::new();
    v4.insert(
        "tcp".to_string(),
        ServerSettings {
            name: "tcp".to_string(),
            listen: SocketAddr::new(bind_ip, port),
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
    config.v4 = v4;

    let mut ws_listeners = HashMap::new();
    ws_listeners.insert(
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
    config.ws = Some(ws_listeners);

    config.console.listen = format!("127.0.0.1:{}", port + 1);

    tracing::info!(
        "MQTT Broker starting — TCP {}:{}, WS 127.0.0.1:{}, storage: {}",
        bind_ip,
        port,
        ws_port,
        storage_path
    );

    let mut broker = Broker::new(config);
    broker.start().unwrap();

    loop {
        std::thread::park();
    }
}
