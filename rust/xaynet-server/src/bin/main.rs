use xaynet_server::app::bootstrap;

#[tokio::main]
async fn main() {
    bootstrap().await
}
