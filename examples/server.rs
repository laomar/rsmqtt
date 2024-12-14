use rsmqtt::{ConnAck, Connect, Error, MqttServer, Packet, PubAck, Publish};
use tokio::signal;

fn connect(c: Connect) -> Result<Packet, Error> {
    println!("connect hook: {:?}", c);
    Ok(Packet::ConnAck(ConnAck::default()))
}
fn publish(p: Publish) -> Result<Packet, Error> {
    println!("Publish hook: {:?}", p);
    Ok(Packet::PubAck(PubAck::default()))
}
#[tokio::main]
async fn main() {
    MqttServer::new()
        .tcp("0.0.0.0:1883")
        .tls(
            "0.0.0.0:8883",
            "./examples/rsmqtt.crt",
            "./examples/rsmqtt.key",
        )
        .ws("0.0.0.0:8083")
        .wss(
            "0.0.0.0:8084",
            "./examples/rsmqtt.crt",
            "./examples/rsmqtt.key",
        )
        .proxy_protocol(true)
        .connect(connect)
        .publish(publish)
        .run()
        .await
        .unwrap();
    signal::ctrl_c().await.expect("ctrl-c pressed");
    println!("Mqtt server stopped");
}
