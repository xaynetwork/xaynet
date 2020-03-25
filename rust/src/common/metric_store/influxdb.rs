use chrono::{DateTime, Utc};
use influxdb::{Client, InfluxDbWriteable, Timestamp};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

#[derive(InfluxDbWriteable)]
struct RoundMeasurement {
    time: DateTime<Utc>,
    round: u32,
}

#[derive(InfluxDbWriteable)]
struct CounterMeasurement {
    time: DateTime<Utc>,
    number_of_selected_participants: u32,
    number_of_waiting_participants: u32,
    number_of_done_participants: u32,
    number_of_done_inactive_participants: u32,
    number_of_ignored_participants: u32,
}

trait Metric {
    fn to_measurement(&self) -> Box<dyn InfluxDbWriteable>;
}

pub async fn run_metricstore(mut influx_connector: InfluxDBConnector) {
    loop {
        match influx_connector.receiver.recv().await {
            Some(t) => {
                // let mut write_query: WriteQuery =
                //     Query::write_query(Timestamp::Now, metric_owner.to_string());

                // for (name, value) in fields {
                //     write_query = write_query.add_field(name, value);
                // }

                // // Submit the query to InfluxDB.
                // let _ = influx_connector
                //     .client
                //     .query(&write_query)
                //     .await
                //     .map_err(|e| error!("{}", e));
            }
            None => {
                warn!("All senders have been dropped!");
                return;
            }
        }
    }
}

// pub enum MetricOwner {
//     Coordinator,
//     Participant,
// }

// impl From<&MetricOwner> for &'static str {
//     fn from(metric_owner: &MetricOwner) -> &'static str {
//         match metric_owner {
//             MetricOwner::Coordinator => "coordinator",
//             MetricOwner::Participant => "participant",
//         }
//     }
// }

// impl ToString for MetricOwner {
//     fn to_string(&self) -> String {
//         Into::<&str>::into(self).into()
//     }
// }

// pub struct Metric(pub MetricOwner, pub Vec<(&'static str, Type)>);

pub struct InfluxDBConnector {
    client: Client,
    receiver: UnboundedReceiver<Box<dyn Metric>>,
}

impl InfluxDBConnector {
    pub fn new(host: &str, db_name: &str) -> (InfluxDBConnector, UnboundedSender<Box<dyn Metric>>) {
        let (sender, receiver) = unbounded_channel();
        (
            InfluxDBConnector {
                client: Client::new(host, db_name),
                receiver,
            },
            sender,
        )
    }
}
