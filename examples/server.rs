use rsmqtt::Mqtt;
use tokio::signal;
#[tokio::main]
async fn main() {
    tokio::task::spawn(async { Mqtt::new().run().await });
    tokio::task::spawn(async {
        Mqtt::new()
            .tls(
                "0.0.0.0:8883",
                "./examples/rsmqtt.crt",
                "./examples/rsmqtt.key",
            )
            .run()
            .await
    });
    tokio::task::spawn(async { Mqtt::new().ws("0.0.0.0:8083", "/ws").run().await });
    tokio::task::spawn(async {
        Mqtt::new()
            .wss(
                "0.0.0.0:8084",
                "/wss",
                "./examples/rsmqtt.crt",
                "./examples/rsmqtt.key",
            )
            .run()
            .await
    });
    signal::ctrl_c().await.unwrap();
    println!("ctrl-c pressed");
}
