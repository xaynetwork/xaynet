use influxdb::WriteQuery;

#[derive(Debug, Clone)]
pub struct MetricsSender();

impl MetricsSender {
    pub fn send(&mut self, _query: WriteQuery) {}
}
