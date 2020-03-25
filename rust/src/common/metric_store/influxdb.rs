use chrono::{DateTime, Utc};
use influxdb::{Client, InfluxDbWriteable, Timestamp};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

pub enum Measurement {
    Round(RoundMeasurement),
    Counters(CountersMeasurement),
}

#[derive(InfluxDbWriteable)]
pub struct RoundMeasurement {
    time: DateTime<Utc>,
    round: u32,
}

impl RoundMeasurement {
    pub fn new(round: u32) -> RoundMeasurement {
        RoundMeasurement {
            time: Timestamp::Now.into(),
            round,
        }
    }
}

impl From<RoundMeasurement> for Measurement {
    fn from(value: RoundMeasurement) -> Self {
        Self::Round(value)
    }
}

#[derive(InfluxDbWriteable)]
pub struct CountersMeasurement {
    time: DateTime<Utc>,
    number_of_selected_participants: u32,
    number_of_waiting_participants: u32,
    number_of_done_participants: u32,
    number_of_done_inactive_participants: u32,
    number_of_ignored_participants: u32,
}

impl CountersMeasurement {
    pub fn new(
        number_of_selected_participants: u32,
        number_of_waiting_participants: u32,
        number_of_done_participants: u32,
        number_of_done_inactive_participants: u32,
        number_of_ignored_participants: u32,
    ) -> CountersMeasurement {
        CountersMeasurement {
            time: Timestamp::Now.into(),
            number_of_selected_participants,
            number_of_waiting_participants,
            number_of_done_participants,
            number_of_done_inactive_participants,
            number_of_ignored_participants,
        }
    }
}

impl From<CountersMeasurement> for Measurement {
    fn from(value: CountersMeasurement) -> Self {
        Self::Counters(value)
    }
}

pub async fn run_metricstore(mut influxdb_connector: InfluxDBConnector) {
    loop {
        match influxdb_connector.receiver.recv().await {
            Some(measurement) => {
                let query = match measurement {
                    Measurement::Round(round) => round.into_query("coordinator"),
                    Measurement::Counters(counter) => counter.into_query("coordinator"),
                };
                let _ = influxdb_connector
                    .client
                    .query(&query)
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

pub struct InfluxDBConnector {
    client: Client,
    receiver: UnboundedReceiver<Measurement>,
}

impl InfluxDBConnector {
    pub fn new(host: &str, db_name: &str) -> (InfluxDBConnector, UnboundedSender<Measurement>) {
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
