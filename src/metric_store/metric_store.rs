extern crate influent;

use influent::client::http::HttpClient;
use influent::client::{Client, Credentials, ClientError};
use influent::create_client;
use influent::measurement::{Measurement, Value};

pub enum MetricOwner {
    Coordinator,
    Participant,
}

pub trait MetricStore {
    fn write(&self, metrics_owner: MetricOwner, fields: Vec<(String, Value)>);
    fn write_with_tags(
        &self,
        metrics_owner: MetricOwner,
        fields: Vec<(String, Value)>,
        tags: Vec<(String, String)>,
    );
}

impl InfluxDBMetricStore<'_> {
    pub fn new<'a>(credentials: Credentials<'a>, hosts: Vec<&'a str>) -> InfluxDBMetricStore<'a> {
        InfluxDBMetricStore {
            client: create_client(credentials, hosts),
        }
    }

    fn metrics_owner_to_string(&self, metrics_owner: MetricOwner) -> &'static str {
        match metrics_owner {
            MetricOwner::Coordinator => "coordinator",
            MetricOwner::Participant => "participant",
        }
    }
}

pub struct InfluxDBMetricStore<'a> {
    client: HttpClient<'a>,
}

impl MetricStore for InfluxDBMetricStore<'_> {
    fn write(&self, metrics_owner: MetricOwner, fields: Vec<(String, Value)>) {
        let mut measurement = Measurement::new(self.metrics_owner_to_string(metrics_owner));
        for (name, value) in fields {
            measurement.add_field(name, value);
        }

        self.client.write_one(measurement, None);
    }

    fn write_with_tags(
        &self,
        metrics_owner: MetricOwner,
        fields: Vec<(String, Value)>,
        tags: Vec<(String, String)>,
    ) {
        let mut measurement = Measurement::new(self.metrics_owner_to_string(metrics_owner));
        for (name, value) in fields {
            measurement.add_field(name, value);
        }
        for (name, value) in tags {
            measurement.add_tag(name, value);
        }

        self.client.write_one(measurement, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write() {
        let credentials = Credentials {
            username: "root",
            password: "root",
            database: "metrics",
        };
        let hosts = vec!["http://localhost:8086"];

        let metric_store = InfluxDBMetricStore::new( credentials, hosts);
        let fields = vec![(String::from("CPU"), Value::Integer(123))];
        metric_store.write(MetricOwner::Coordinator, fields);
    }

    #[test]
    fn write_with_tags() {
        let credentials = Credentials {
            username: "root",
            password: "root",
            database: "metrics",
        };
        let hosts = vec!["http://localhost:8086"];

        let metric_store = InfluxDBMetricStore::new( credentials, hosts);
        let fields = vec![(String::from("CPU"), Value::Integer(123))];
        let tags = vec![(String::from("ID"), String::from("1234-1234-1234-1234"))];
        metric_store.write_with_tags(MetricOwner::Coordinator, fields, tags);
    }
}
