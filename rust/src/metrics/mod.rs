use influxdb::{Client, WriteQuery};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

mod models;

pub mod round_parameters {
    use super::models::{Measurement, RoundParamSum, RoundParamUpdate};
    use influxdb::{InfluxDbWriteable, Timestamp, WriteQuery};
    pub mod sum {
        use super::*;

        pub fn update(sum: f64) -> WriteQuery {
            RoundParamSum {
                time: Timestamp::Now.into(),
                round_param_sum: sum,
            }
            .into_query(Measurement::StateMachine.to_string())
        }
    }

    pub mod update {
        use super::*;

        pub fn update(update: f64) -> WriteQuery {
            RoundParamUpdate {
                time: Timestamp::Now.into(),
                round_param_update: update,
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
                message_sum: 1,
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
                message_update: 1,
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
                message_sum2: 1,
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
                message_discarded: 1,
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
                message_rejected: 1,
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::state_machine::{
        phases::{PhaseName, StateError},
        RoundFailed,
    };
    use influxdb::Query;

    // The fields of the WriteQuery are private and there are no kinds of getters for the fields.
    // One way to get something is via `build`.
    // We are using `contains` because we don't to check the timestamp at the end.
    // e.g. "state_machine sum=0.6 1596463830315036000"

    #[test]
    fn test_round_parameters_sum() {
        let query = round_parameters::sum::update(0.6).build();
        assert!(format!("{:?}", query.unwrap()).contains("state_machine round_param_sum=0.6"));
    }

    #[test]
    fn test_round_parameters_update() {
        let query = round_parameters::update::update(0.8).build();
        assert!(format!("{:?}", query.unwrap()).contains("state_machine round_param_update=0.8"));
    }

    #[test]
    fn test_phase_name() {
        let query = phase::update(PhaseName::Idle).build();
        assert!(format!("{:?}", query.unwrap()).contains("state_machine phase=0"));

        let query = phase::update(PhaseName::Sum).build();
        assert!(format!("{:?}", query.unwrap()).contains("state_machine phase=1"));

        let query = phase::update(PhaseName::Update).build();
        assert!(format!("{:?}", query.unwrap()).contains("state_machine phase=2"));

        let query = phase::update(PhaseName::Sum2).build();
        assert!(format!("{:?}", query.unwrap()).contains("state_machine phase=3"));

        let query = phase::update(PhaseName::Unmask).build();
        assert!(format!("{:?}", query.unwrap()).contains("state_machine phase=4"));

        let query = phase::update(PhaseName::Error).build();
        assert!(format!("{:?}", query.unwrap()).contains("state_machine phase=5"));

        let query = phase::update(PhaseName::Shutdown).build();
        assert!(format!("{:?}", query.unwrap()).contains("state_machine phase=6"));
    }

    #[test]
    fn test_phase_error() {
        let query = phase::error::emit(&StateError::RoundError(RoundFailed::NoMask)).build();
        assert!(format!("{:?}", query.unwrap()).contains(
            "event title=\\\"state\\\\ failed:\\\\ round\\\\ error:\\\\ no\\\\ mask\\\\ found\\\""
        ));
    }

    #[test]
    fn test_masks_total_number() {
        let query = masks::total_number::update(12).build();
        assert!(format!("{:?}", query.unwrap()).contains("state_machine masks_total_number=12"));
    }

    #[test]
    fn test_round_total_number() {
        let query = round::total_number::update(2).build();
        assert!(format!("{:?}", query.unwrap()).contains("state_machine round_total_number=2"));
    }

    #[test]
    fn test_round_successful() {
        let query = round::successful::increment().build();
        assert!(format!("{:?}", query.unwrap()).contains("state_machine round_successful=1"));
    }

    #[test]
    fn test_message_sum() {
        let query = message::sum::increment(1, PhaseName::Sum).build();
        assert!(format!("{:?}", query.unwrap())
            .contains("state_machine,round_id=\\\"1\\\",phase=\\\"1\\\" message_sum=1"));
    }

    #[test]
    fn test_message_update() {
        let query = message::update::increment(1, PhaseName::Update).build();
        assert!(format!("{:?}", query.unwrap())
            .contains("state_machine,round_id=\\\"1\\\",phase=\\\"2\\\" message_update=1"));
    }

    #[test]
    fn test_message_sum2() {
        let query = message::sum2::increment(1, PhaseName::Sum2).build();
        assert!(format!("{:?}", query.unwrap())
            .contains("state_machine,round_id=\\\"1\\\",phase=\\\"3\\\" message_sum2=1"));
    }

    #[test]
    fn test_message_discarded() {
        let query = message::discarded::increment(1, PhaseName::Idle).build();
        assert!(format!("{:?}", query.unwrap())
            .contains("state_machine,round_id=\\\"1\\\",phase=\\\"0\\\" message_discarded=1"));
    }

    #[test]
    fn test_message_rejected() {
        let query = message::rejected::increment(1, PhaseName::Sum).build();
        assert!(format!("{:?}", query.unwrap())
            .contains("state_machine,round_id=\\\"1\\\",phase=\\\"1\\\" message_rejected=1"));
    }
}
