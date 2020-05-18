use xain_fl::service::Service;

#[tokio::main]
async fn main() {
    let (service, _handle) = Service::new().unwrap();
    service.await;
}
