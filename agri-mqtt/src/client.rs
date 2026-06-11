use anyhow::Result;
use rumqttc::{AsyncClient, EventLoop, MqttOptions, QoS};
use tracing::info;

pub fn create_client(
    broker_addr: &str,
    port: u16,
    client_id: &str,
) -> Result<(AsyncClient, EventLoop)> {
    let mut mqtt_options = MqttOptions::new(client_id, broker_addr, port);
    mqtt_options.set_keep_alive(std::time::Duration::from_secs(5));
    mqtt_options.set_clean_session(false);

    let (client, eventloop) = AsyncClient::new(mqtt_options, 10);

    info!("MQTT client created: {}", client_id);

    Ok((client, eventloop))
}

pub async fn publish_command(
    client: &AsyncClient,
    node_id: &str,
    cmd_id: &str,
    payload: &str,
) -> Result<()> {
    let topic = agri_core::topics::command_topic(node_id, cmd_id);

    client
        .publish(&topic, QoS::AtLeastOnce, false, payload.as_bytes())
        .await?;

    info!("Published command to topic: {}", topic);

    Ok(())
}
