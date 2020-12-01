use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

use crate::{
    state_machine::{
        phases::Sum2,
        IntoPhase,
        Phase,
        PhaseIo,
        Progress,
        State,
        Step,
        TransitionOutcome,
        IO,
    },
    MessageEncoder,
};

/// Sending message phase data
#[derive(Serialize, Deserialize, Debug)]
pub struct Sending {
    /// The message to send
    message: MessageEncoder,

    /// Chunk that couldn't be sent and should be tried again
    failed: Option<Vec<u8>>,

    /// State of the phase to transition to, after this one completes
    next: Next,
}

#[derive(Serialize, Deserialize, Debug)]
enum Next {
    Sum2(Sum2),
    Awaiting,
}

impl Sending {
    fn new(message: MessageEncoder, next: Next) -> Self {
        Self {
            message,
            failed: None,
            next,
        }
    }
    pub fn from_sum(message: MessageEncoder, next: Sum2) -> Self {
        Self::new(message, Next::Sum2(next))
    }

    pub fn from_update(message: MessageEncoder) -> Self {
        Self::new(message, Next::Awaiting)
    }

    pub fn from_sum2(message: MessageEncoder) -> Self {
        Self::new(message, Next::Awaiting)
    }
}

impl IntoPhase<Sending> for State<Sending> {
    fn into_phase(self, io: PhaseIo) -> Phase<Sending> {
        Phase::<_>::new(self, io)
    }
}

impl Phase<Sending> {
    async fn try_send(mut self, data: Vec<u8>) -> Progress<Sending> {
        info!("sending message (size = {})", data.len());
        if let Err(e) = self.io.send_message(data.clone()).await {
            error!("failed to send message: {:?}", e);
            self.state.private.failed = Some(data);
            Progress::Stuck(self)
        } else {
            Progress::Updated(self.into())
        }
    }

    async fn send_some(mut self) -> Progress<Sending> {
        if let Some(data) = self.state.private.failed.take() {
            debug!("retrying to send message that couldn't be send previously");
            self.try_send(data).await
        } else {
            match self.state.private.message.next() {
                Some(data) => {
                    let data = self.state.shared.round_params.pk.encrypt(data.as_slice());
                    self.try_send(data).await
                }
                None => {
                    debug!("nothing left to send");
                    Progress::Continue(self)
                }
            }
        }
    }
}

#[async_trait]
impl Step for Phase<Sending> {
    async fn step(mut self) -> TransitionOutcome {
        info!("sending task");
        self = try_progress!(self.send_some().await);
        info!("done sending");
        match self.state.private.next {
            Next::Sum2(sum2) => {
                let state = State::new(self.state.shared, sum2);
                TransitionOutcome::Complete(state.into_phase(self.io).into())
            }
            Next::Awaiting => {
                let phase = self.into_awaiting();
                TransitionOutcome::Complete(phase.into())
            }
        }
    }
}
