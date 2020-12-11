use super::{Dispatcher, Request};
use tower::{buffer::Buffer, limit::ConcurrencyLimit, load_shed::LoadShed, ServiceBuilder};

pub(in crate::metrics) struct InfluxDbService(
    pub LoadShed<Buffer<ConcurrencyLimit<Dispatcher>, Request>>,
);

impl InfluxDbService {
    pub fn new(dispatcher: Dispatcher) -> Self {
        let service = ServiceBuilder::new()
            .load_shed()
            .buffer(4048)
            .concurrency_limit(50)
            .service(dispatcher);
        Self(service)
    }
}
