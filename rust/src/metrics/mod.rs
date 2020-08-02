use influxdb::{Client, WriteQuery};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

mod models;

pub mod round_parameters {
    use super::models::{Measurement, Sum, Update};
    use influxdb::{InfluxDbWriteable, Timestamp, WriteQuery};
    pub mod sum {
        use super::*;

        pub fn update(sum: f64) -> WriteQuery {
            Sum {
                time: Timestamp::Now.into(),
                sum,
            }
            .into_query(Measurement::StateMachine.to_string())
        }
    }

    pub mod update {
        use super::*;

        pub fn update(update: f64) -> WriteQuery {
            Update {
                time: Timestamp::Now.into(),
                update,
            }
            .into_query(Measurement::StateMachine.to_string())
        }
    }
}

pub mod phase {
    use super::models::{Event, Measurement, Phase};
    use crate::state_machine::phases::{PhaseName, StateError};
    use influxdb::{InfluxDbWriteable, Timestamp, WriteQuery};
    pub mod error {
        use super::*;

        pub fn emit(error: &StateError) -> WriteQuery {
            Event {
                time: Timestamp::Now.into(),
                title: error.to_string(),
                text: None,
                tags: None,
            }
            .into_query(Measurement::Event.to_string())
        }
    }

    pub fn update(phase: PhaseName) -> WriteQuery {
        Phase {
            time: Timestamp::Now.into(),
            phase: phase as u8,
        }
        .into_query(Measurement::StateMachine.to_string())
    }
}

pub mod masks {
    use super::models::{MasksTotalNumber, Measurement};
    use influxdb::{InfluxDbWriteable, Timestamp, WriteQuery};
    pub mod total_number {
        use super::*;

        pub fn update(total_number: usize) -> WriteQuery {
            MasksTotalNumber {
                time: Timestamp::Now.into(),
                masks_total_number: total_number as u64,
            }
            .into_query(Measurement::StateMachine.to_string())
        }
    }
}

pub mod round {
    use super::models::{Measurement, RoundSuccessful, RoundTotalNumber};
    use influxdb::{InfluxDbWriteable, Timestamp, WriteQuery};
    pub mod total_number {
        use super::*;

        pub fn update(total_number: u64) -> WriteQuery {
            RoundTotalNumber {
                time: Timestamp::Now.into(),
                round_total_number: total_number,
            }
            .into_query(Measurement::StateMachine.to_string())
        }
    }

    pub mod successful {
        use super::*;

        pub fn increment() -> WriteQuery {
            RoundSuccessful {
                time: Timestamp::Now.into(),
                round_successful: 1,
            }
            .into_query(Measurement::StateMachine.to_string())
        }
    }
}

pub mod message {
    use super::models::{
        Measurement,
        MessageDiscarded,
        MessageRejected,
        MessageSum,
        MessageSum2,
        MessageUpdate,
    };
    use crate::state_machine::phases::PhaseName;
    use influxdb::{InfluxDbWriteable, Timestamp, WriteQuery};
    pub mod sum {
        use super::*;

        pub fn increment(round_id: u64, phase: PhaseName) -> WriteQuery {
            MessageSum {
                time: Timestamp::Now.into(),
                sum: 1,
                round_id,
                phase: phase as u8,
            }
            .into_query(Measurement::StateMachine.to_string())
        }
    }

    pub mod update {
        use super::*;

        pub fn increment(round_id: u64, phase: PhaseName) -> WriteQuery {
            MessageUpdate {
                time: Timestamp::Now.into(),
                update: 1,
                round_id,
                phase: phase as u8,
            }
            .into_query(Measurement::StateMachine.to_string())
        }
    }

    pub mod sum2 {
        use super::*;

        pub fn increment(round_id: u64, phase: PhaseName) -> WriteQuery {
            MessageSum2 {
                time: Timestamp::Now.into(),
                sum2: 1,
                round_id,
                phase: phase as u8,
            }
            .into_query(Measurement::StateMachine.to_string())
        }
    }

    pub mod discarded {
        use super::*;

        pub fn increment(round_id: u64, phase: PhaseName) -> WriteQuery {
            MessageDiscarded {
                time: Timestamp::Now.into(),
                discarded: 1,
                round_id,
                phase: phase as u8,
            }
            .into_query(Measurement::StateMachine.to_string())
        }
    }

    pub mod rejected {
        use super::*;

        pub fn increment(round_id: u64, phase: PhaseName) -> WriteQuery {
            MessageRejected {
                time: Timestamp::Now.into(),
                rejected: 1,
                round_id,
                phase: phase as u8,
            }
            .into_query(Measurement::StateMachine.to_string())
        }
    }
}

pub async fn run_metric_service(mut metics_service: MetricsService) {
    loop {
        match metics_service.receiver.recv().await {
            Some(write_query) => {
                let _ = metics_service
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

pub struct MetricsSender(UnboundedSender<WriteQuery>);

impl MetricsSender {
    pub fn send(&self, query: WriteQuery) {
        let _ = self.0.send(query).map_err(|e| error!("{}", e));
    }
}

pub struct MetricsService {
    client: Client,
    receiver: UnboundedReceiver<WriteQuery>,
}

impl MetricsService {
    pub fn new(host: &str, db_name: &str) -> (MetricsService, MetricsSender) {
        let (sender, receiver) = unbounded_channel();
        (
            MetricsService {
                client: Client::new(host, db_name),
                receiver,
            },
            MetricsSender(sender),
        )
    }
}
