use async_trait::async_trait;
use paste::paste;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

use crate::{
    state_machine::{
        phases::Sum2,
        Awaiting,
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

/// Implements the `SendingSum`, `SendingUpdate` and `SendingSum2` phases and transitions.
macro_rules! impl_sending {
    ($Phase: ty, $Next: ty, $phase: expr, $next: expr) => {
        paste! {
            #[doc = "The state of the " $phase " sending phase."]
            #[derive(Serialize, Deserialize, Debug)]
            pub struct [<Sending $Phase>] {
                /// The message to send.
                message: MessageEncoder,

                /// Chunk that couldn't be sent and should be tried again.
                failed: Option<Vec<u8>>,

                /// State of the phase to transition to, after this one completes.
                next: $Next,
            }

            impl [<Sending $Phase>] {
                #[doc = "Creates a new " $phase " sending state."]
                pub fn new(message: MessageEncoder, next: $Next) -> Self {
                    Self {
                        message,
                        failed: None,
                        next,
                    }
                }
            }

            impl IntoPhase<[<Sending $Phase>]> for State<[<Sending $Phase>]> {
                fn into_phase(self, io: PhaseIo) -> Phase<[<Sending $Phase>]> {
                    Phase::<_>::new(self, io)
                }
            }

            #[async_trait]
            impl Step for Phase<[<Sending $Phase>]> {
                async fn step(mut self) -> TransitionOutcome {
                    info!("sending {} message", $phase);
                    self = try_progress!(self.send_next().await);

                    info!("done sending {} message, going to {} phase", $phase, $next);
                    let phase: Phase<$Next> = self.into();
                    TransitionOutcome::Complete(phase.into())
                }
            }

            impl From<Phase<[<Sending $Phase>]>> for Phase<$Next> {
                fn from(sending: Phase<[<Sending $Phase>]>) -> Self {
                    State::new(sending.state.shared, Box::new(sending.state.private.next))
                        .into_phase(sending.io)
                }
            }

            impl Phase<[<Sending $Phase>]> {
                #[doc = "Tries to send a " $phase " message and reports back on the progress made."]
                async fn try_send(mut self, data: Vec<u8>) -> Progress<[<Sending $Phase>]> {
                    info!("sending {} message (size = {})", $phase, data.len());
                    if let Err(e) = self.io.send_message(data.clone()).await {
                        error!("failed to send {} message: {:?}", $phase, e);
                        self.state.private.failed = Some(data);
                        Progress::Stuck(self)
                    } else {
                        Progress::Updated(self.into())
                    }
                }

                #[doc =
                    "Sends the next " $phase " message and reports back on the progress made.\n"
                    "\n"
                    "Retries to send a previously failed message. Otherwise, tries to send the "
                    "next message."
                ]
                async fn send_next(mut self) -> Progress<[<Sending $Phase>]> {
                    if let Some(data) = self.state.private.failed.take() {
                        debug!(
                            "retrying to send {} message that couldn't be sent previously",
                            $phase
                        );
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
        }
    }
}

impl_sending!(Sum, Sum2, "sum", "sum2");
impl_sending!(Update, Awaiting, "update", "awaiting");
impl_sending!(Sum2, Awaiting, "sum2", "awaiting");
