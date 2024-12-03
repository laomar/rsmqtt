use rsmqtt::MqttServer;
use tokio::signal;
use rsmqtt::hook::HookType;

fn hook() {}
#[tokio::main]
async fn main() {
    MqttServer::new()
        .listen("tcp", "0.0.0.0:1883")
        .listens(
            "tls",
            "0.0.0.0:8883",
            "./examples/rsmqtt.crt",
            "./examples/rsmqtt.key",
        )
        .listen("ws", "0.0.0.0:8083")
        .listens(
            "wss",
            "0.0.0.0:8084",
            "./examples/rsmqtt.crt",
            "./examples/rsmqtt.key",
        )
        .hook(HookType::Connect)
        .run()
        .await
        .unwrap();
    signal::ctrl_c().await.expect("ctrl-c pressed");
    println!("Mqtt server stopped");
}
