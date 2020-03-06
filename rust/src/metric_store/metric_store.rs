use influxdb::{Client, Query, Timestamp, Type, WriteQuery};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

pub enum MetricOwner {
    Coordinator,
    Participant,
}

impl From<&MetricOwner> for &'static str {
    fn from(metric_owner: &MetricOwner) -> &'static str {
        match metric_owner {
            MetricOwner::Coordinator => "coordinator",
            MetricOwner::Participant => "participant",
        }
    }
}

impl ToString for MetricOwner {
    fn to_string(&self) -> String {
        Into::<&str>::into(self).into()
    }
}

pub struct InfluxDBMetricStore {
    client: Client,
    receiver: UnboundedReceiver<Metric>,
}

pub async fn run_metricstore(mut influx_client: InfluxDBMetricStore) {
    loop {
        match influx_client.receiver.recv().await {
            Some(Metric(metric_owner, fields)) => {
                let mut write_query: WriteQuery =
                    Query::write_query(Timestamp::Now, metric_owner.to_string());

                for (name, value) in fields {
                    write_query = write_query.add_field(name, value);
                }

                // Submit the query to InfluxDB.
                let _ = influx_client
                    .client
                    .query(&write_query)
                    .await
                    .map_err(|e| error!("{}", e));
            }
            None => {
                warn!("All senders have been dropped!");
                return;
            }
        }
    }
}

pub struct Metric(pub MetricOwner, pub Vec<(&'static str, Type)>);

impl InfluxDBMetricStore {
    pub fn new(host: &str, db_name: &str) -> (InfluxDBMetricStore, UnboundedSender<Metric>) {
        let (sender, receiver) = unbounded_channel();
        (
            InfluxDBMetricStore {
                client: Client::new(host, db_name),
                receiver,
            },
            sender,
        )
    }
}
