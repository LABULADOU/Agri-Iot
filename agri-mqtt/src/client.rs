use anyhow::Result;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use tracing::info;

pub async fn create_client(
    broker_addr: &str,
    port: u16,
    client_id: &str,
) -> Result<AsyncClient> {
    let mut mqtt_options = MqttOptions::new(client_id, broker_addr, port);
    mqtt_options.set_keep_alive(std::time::Duration::from_secs(5));
    mqtt_options.set_clean_session(true);

    let (client, mut eventloop) = AsyncClient::new(mqtt_options, 10);

    info!("MQTT client created: {}", client_id);

    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(notification) => {
                    tracing::debug!("MQTT notification: {:?}", notification);
                }
                Err(e) => {
                    tracing::warn!("MQTT eventloop error: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    });

    Ok(client)
}

pub async fn publish_command(
    client: &AsyncClient,
    node_id: &str,
    cmd_id: &str,
    payload: &str,
) -> Result<()> {
    let topic = format!("agri/node/{}/command/{}", node_id, cmd_id);

    client
        .publish(&topic, QoS::AtLeastOnce, false, payload.as_bytes())
        .await?;

    info!("Published command to topic: {}", topic);

    Ok(())
}
