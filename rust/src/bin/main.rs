use tracing_subscriber::*;
use xain_fl::{rest, service::Service};

#[tokio::main]
async fn main() {
    let _fmt_subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .init();

    let (service, handle) = Service::new().unwrap();

    tokio::select! {
        _ = service => {
            println!("shutting down: Service terminated");
        }
        _ = rest::serve(([127, 0, 0, 1], 3030), handle.clone()) => {
            println!("shutting down: REST server terminated");
        }
    }
}
