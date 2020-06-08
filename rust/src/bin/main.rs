use xain_fl::service::Service;
use xain_fl::rest;

#[tokio::main]
async fn main() {
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
