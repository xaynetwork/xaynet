use super::{CoordinatorState, RedisStore, State, StateError, StateMachine};
use crate::{coordinator::ProtocolEvent, coordinator_async::idle::Idle};
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct Error {
    error: StateError,
}

impl State<Error> {
    pub fn new(
        coordinator_state: CoordinatorState,
        message_rx: mpsc::UnboundedReceiver<Vec<u8>>,
        redis: RedisStore,
        events_rx: mpsc::UnboundedSender<ProtocolEvent>,
        error: StateError,
    ) -> StateMachine {
        info!("state transition");
        StateMachine::Error(Self {
            _inner: Error { error },
            coordinator_state,
            message_rx,
            redis,
            events_rx,
        })
    }

    pub async fn next(self) -> StateMachine {
        error!("state transition failed! error: {:?}", self._inner.error);
        info!("restart round");
        self.emit_end_round();
        State::<Idle>::new(
            self.coordinator_state,
            self.message_rx,
            self.redis,
            self.events_rx,
        )
    }

    fn emit_end_round(&self) {
        let _ = self.events_rx.send(ProtocolEvent::EndRound(None));
    }
}
