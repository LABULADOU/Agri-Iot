use rumqttc::{AsyncClient, MqttOptions, QoS, Event, Packet};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    let mut mqttopts = MqttOptions::new("rust-test-sub", "127.0.0.1", 1883);
    mqttopts.set_clean_session(true);
    mqttopts.set_keep_alive(Duration::from_secs(30));

    let (client, mut eventloop) = AsyncClient::new(mqttopts, 100);
    client.subscribe("#", QoS::AtLeastOnce).await.unwrap();
    println!("[SUB] Subscribed to #");

    // publish after a delay from another client
    tokio::spawn(async move {
        sleep(Duration::from_secs(2)).await;
        let mut pubopts = MqttOptions::new("rust-test-pub", "127.0.0.1", 1883);
        pubopts.set_clean_session(true);
        pubopts.set_keep_alive(Duration::from_secs(30));
        let (pub_client, mut pub_el) = AsyncClient::new(pubopts, 100);
        sleep(Duration::from_secs(1)).await;
        pub_client
            .publish(
                "agri/node/esp32-node-001/telemetry",
                QoS::AtLeastOnce,
                false,
                "{\"rust_test\":true}",
            )
            .await
            .unwrap();
        println!("[PUB] Published");
        tokio::spawn(async move { loop { pub_el.poll().await.ok(); } });
    });

    for i in 0..15 {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::Publish(p))) => {
                println!(
                    "[SUB] GOT: topic={} payload={:?}",
                    p.topic,
                    String::from_utf8_lossy(&p.payload)
                );
            }
            Ok(Event::Incoming(Packet::ConnAck(_))) => println!("[SUB] ConnAck"),
            Ok(Event::Incoming(Packet::SubAck(_))) => println!("[SUB] SubAck"),
            Ok(Event::Incoming(ev)) => println!("[SUB] Other: {:?}", ev),
            Ok(Event::Outgoing(_)) => {}
            Err(e) => println!("[SUB] Error: {:?}", e),
        }
        sleep(Duration::from_secs(1)).await;
    }
    println!("Done");
}
