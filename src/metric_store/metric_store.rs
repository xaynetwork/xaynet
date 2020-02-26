extern crate influent;

use core::future::Future;
use influxdb::{Client, Error, Query, Timestamp, Type, WriteQuery};

pub enum MetricOwner {
    Coordinator,
    Participant,
}

impl InfluxDBMetricStore {
    pub fn new(host: &str, db_name: &str) -> InfluxDBMetricStore {
        InfluxDBMetricStore {
            client: Client::new(host, db_name),
        }
    }

    fn metrics_owner_to_string(&self, metrics_owner: MetricOwner) -> &'static str {
        match metrics_owner {
            MetricOwner::Coordinator => "coordinator",
            MetricOwner::Participant => "participant",
        }
    }

    async fn write(&self, metrics_owner: MetricOwner, fields: Vec<(String, Type)>) -> () {
        let write_query: WriteQuery =
            Query::write_query(Timestamp::Now, self.metrics_owner_to_string(metrics_owner));
        let write_query = write_query.add_field("d", 12);

        // for (name, value) in fields {
        //     let write_query = write_query.add_field(name, value);
        // };

        // // Submit the query to InfluxDB.
        let res = self.client.query(&write_query).await;
    }

    //  fn write_with_tags(
    //     &self,
    //     metrics_owner: MetricOwner,
    //     fields: Vec<(String, Type)>,
    //     tags: Vec<(String, String)>,
    // ) -> Future<Output = Type>{
    //     let mut write_query = Query::write_query(Timestamp::Now, self.metrics_owner_to_string(metrics_owner));

    //     for (name, value) in fields {
    //         write_query.add_field(name, value);
    //     }

    //     for (name, value) in tags {
    //         write_query.add_tag(name, value);
    //     }

    // Submit the query to InfluxDB.
    // self.client.query(&write_query)
    // }
}

pub struct InfluxDBMetricStore {
    client: Client,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write() {
        // let credentials = Credentials {
        //     username: "root",
        //     password: "root",
        //     database: "metrics",
        // };
        // let hosts = vec!["http://localhost:8086"];

        // let metric_store = InfluxDBMetricStore::new(credentials, hosts);
        // let fields = vec![(String::from("CPU"), Value::Integer(123))];
        // metric_store.write(MetricOwner::Coordinator, fields);
    }

    #[test]
    fn write_with_tags() {
        // let credentials = Credentials {
        //     username: "root",
        //     password: "root",
        //     database: "metrics",
        // };
        // let hosts = vec!["http://localhost:8086"];

        // let metric_store = InfluxDBMetricStore::new(credentials, hosts);
        // let fields = vec![(String::from("CPU"), Value::Integer(123))];
        // let tags = vec![(String::from("ID"), String::from("1234-1234-1234-1234"))];
        // metric_store.write_with_tags(MetricOwner::Coordinator, fields, tags);
    }
}
