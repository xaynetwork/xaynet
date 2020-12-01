use async_trait::async_trait;
use derive_more::From;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, error, info, warn};

use super::{Awaiting, NewRound, Sending, Sum, Sum2, Update, IO};
use crate::{
    settings::{MaxMessageSize, PetSettings},
    state_machine::{StateMachine, TransitionOutcome},
    MessageEncoder,
};
use xaynet_core::{
    common::{RoundParameters, RoundSeed},
    crypto::{ByteObject, PublicEncryptKey, SigningKeyPair},
    mask::{self, DataType, MaskConfig, Model},
    message::Payload,
};

/// State of the state machine
#[derive(Debug, Serialize, Deserialize)]
pub struct State<P> {
    /// data specific to the current phase
    pub private: Box<P>,
    /// data common to most of the phases
    pub shared: Box<SharedState>,
}

impl<P> State<P> {
    /// Create a new state
    pub fn new(shared: Box<SharedState>, private: Box<P>) -> Self {
        Self { shared, private }
    }
}

/// A dynamically dispatched [`IO`] object.
pub(crate) type PhaseIo = Box<dyn IO<Model = Box<dyn AsRef<Model> + Send>>>;

/// Represent the state machine in a specific phase
pub struct Phase<P> {
    /// State of the phase.
    pub(super) state: State<P>,
    /// Opaque client for performing IO tasks: talking with the
    /// coordinator API, loading models, etc.
    pub(super) io: PhaseIo,
}

impl<P> std::fmt::Debug for Phase<P>
where
    P: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Phase")
            .field("state", &self.state)
            .field("io", &"PhaseIo")
            .finish()
    }
}

/// Store for all the data that are common to all the phases
#[derive(Serialize, Deserialize, Debug)]
pub struct SharedState {
    /// Keys that identify the participant. They are used to sign the
    /// PET message sent by the participant.
    pub keys: SigningKeyPair,
    /// Scalar used for masking
    pub scalar: f64,
    /// Maximum message size the participant can send. Messages larger
    /// than `message_size` are split in several parts.
    pub message_size: MaxMessageSize,
    /// Current round parameters
    pub round_params: RoundParameters,
}

/// Get arbitrary round parameters. These round parameters are never used, we just
/// temporarily use them in the [`SharedState`] when creating a new state machine. The
/// first thing the state machine does when it runs, is to fetch the real round
/// parameters from the coordinator.
fn dummy_round_parameters() -> RoundParameters {
    RoundParameters {
        pk: PublicEncryptKey::zeroed(),
        sum: 0.0,
        update: 0.0,
        seed: RoundSeed::zeroed(),
        mask_config: MaskConfig {
            group_type: mask::GroupType::Integer,
            data_type: mask::DataType::F32,
            bound_type: mask::BoundType::B0,
            model_type: mask::ModelType::M3,
        }
        .into(),
        model_length: 0,
    }
}

impl SharedState {
    pub fn new(settings: PetSettings) -> Self {
        Self {
            keys: settings.keys,
            scalar: settings.scalar,
            message_size: settings.max_message_size,
            round_params: dummy_round_parameters(),
        }
    }
}

/// A trait that each `Phase<P>` implements. When `Step::step` is called, the phase
/// tries to do a small piece of work.
#[async_trait]
pub trait Step {
    /// Represent an attempt to make progress within a phase. If the step results in a
    /// change in the phase state, the updated state machine is returned as
    /// `TransitionOutcome::Complete`. If no progress can be made, the state machine is
    /// returned unchanged as `TransitionOutcome::Pending`.
    async fn step(mut self) -> TransitionOutcome;
}

#[macro_export]
macro_rules! try_progress {
    ($progress:expr) => {{
        use $crate::state_machine::{Progress, TransitionOutcome};
        match $progress {
            // No progress can be made. Return the state machine as is
            Progress::Stuck(phase) => return TransitionOutcome::Pending(phase.into()),
            // Further progress can be made but require more work, so don't return
            Progress::Continue(phase) => phase,
            // Progress has been made, return the updated state machine
            Progress::Updated(state_machine) => return TransitionOutcome::Complete(state_machine),
        }
    }};
}

/// Represent the presence or absence of progress being made during a phase.
#[derive(Debug)]
pub enum Progress<P> {
    /// No progress can be made currently.
    Stuck(Phase<P>),
    /// More work needs to be done for progress to be made.
    Continue(Phase<P>),
    /// Progress has been made and resulted in this new state machine.
    Updated(StateMachine),
}

impl<P> Phase<P>
where
    Phase<P>: Step + Into<StateMachine>,
{
    /// Try to make some progress in the execution of the PET protocol. There are three
    /// possible outcomes:
    ///
    /// 1. no progress can currently be made and the phase state is unchanged
    /// 2. progress is made but the state machine does not transition to a new
    ///    phase. Internally, the phase state is changed though.
    /// 3. progress is made and the state machine transitions to a new phase.
    ///
    /// In case `1.`, the state machine is returned unchanged, wrapped in
    /// [`TransitionOutcome::Pending`] to indicate to the caller that the state machine
    /// wasn't updated. In case `2.` and `3.` the updated state machine is returned
    /// wrapped in [`TransitionOutcome::Complete`].
    pub async fn step(mut self) -> TransitionOutcome {
        match self.check_round_freshness().await {
            RoundFreshness::Unknown => TransitionOutcome::Pending(self.into()),
            RoundFreshness::Outdated => {
                info!("a new round started: updating the round parameters and resetting the state machine");
                self.io.notify_new_round();
                TransitionOutcome::Complete(
                    Phase::<NewRound>::new(
                        State::new(self.state.shared, Box::new(NewRound)),
                        self.io,
                    )
                    .into(),
                )
            }
            RoundFreshness::Fresh => {
                debug!("round is still fresh, continuing from where we left off");
                <Self as Step>::step(self).await
            }
        }
    }

    /// Check whether the coordinator has published new round parameters. In other
    /// words, this checks whether a new round has started.
    async fn check_round_freshness(&mut self) -> RoundFreshness {
        match self.io.get_round_params().await {
            Err(e) => {
                warn!("failed to fetch round parameters {:?}", e);
                RoundFreshness::Unknown
            }
            Ok(params) => {
                if params == self.state.shared.round_params {
                    debug!("round parameters didn't change");
                    RoundFreshness::Fresh
                } else {
                    info!("fetched fresh round parameters");
                    self.state.shared.round_params = params;
                    RoundFreshness::Outdated
                }
            }
        }
    }
}

/// Trait for building [`Phase<P>`] from a [`State<P>`].
///
/// Note that we could just use [`Phase::new`] for this. However we want to be able to
/// customize the conversion for each phase. For instance, when building a
/// `Phase<Update>` from an `Update`, we want to emit some events with the `io`
/// object. It is cleaner to wrap this custom logic in a trait impl.
pub(crate) trait IntoPhase<P> {
    /// Build the phase with the given `io` object
    fn into_phase(self, io: PhaseIo) -> Phase<P>;
}

impl<P> Phase<P> {
    /// Build a new phase with the given state and io object. This should not be called
    /// directly. Instead, use the [`IntoPhase`] trait to construct a phase.
    pub(crate) fn new(state: State<P>, io: PhaseIo) -> Self {
        Phase { state, io }
    }

    /// Transition to the awaiting phase
    pub fn into_awaiting(self) -> Phase<Awaiting> {
        State::new(self.state.shared, Box::new(Awaiting)).into_phase(self.io)
    }

    /// Transition to the message sending phase
    pub fn message_encoder(&self, payload: Payload) -> MessageEncoder {
        MessageEncoder::new(
            self.state.shared.keys.clone(),
            payload,
            self.state.shared.round_params.pk,
            self.state
                .shared
                .message_size
                .max_payload_size()
                .unwrap_or(0),
        )
        // the encoder rejects Chunk payload, but in the state
        // machine, we never manually create such payloads so
        // unwrapping is fine
        .unwrap()
    }

    /// Send the message created by the given message encoder.
    ///
    /// If the message is split in multiple parts, they are sent sequentially. If a
    /// single part fails, the remaining parts are not sent. There is no retry
    /// mechanism.
    pub async fn send_message(&mut self, encoder: MessageEncoder) -> Result<(), SendMessageError> {
        for part in encoder {
            let data = self.state.shared.round_params.pk.encrypt(part.as_slice());
            self.io.send_message(data).await.map_err(|e| {
                error!("failed to send message: {:?}", e);
                SendMessageError
            })?
        }
        Ok(())
    }
    
    /// Return the local model configuration of the model that is expected in the update phase.
    pub fn local_model_config(&self) -> LocalModelConfig {
        LocalModelConfig {
            data_type: self.state.shared.round_params.mask_config.vect.data_type,
            len: self.state.shared.round_params.model_length,
        }
    }



    #[cfg(test)]
    pub(crate) fn with_io_mock<F>(&mut self, f: F)
    where
        F: FnOnce(&mut super::MockIO),
    {
        let mut mock = super::MockIO::new();
        f(&mut mock);
        self.io = Box::new(mock);
    }

    #[cfg(test)]
    pub(crate) fn check_io_mock(&mut self) {
        // dropping the mock forces the checks to run. We replace it
        // by an empty one, so that we detect if a method is called
        // un-expectedly afterwards
        let _ = std::mem::replace(&mut self.io, Box::new(super::MockIO::new()));
    }
}

#[derive(Debug)]
/// The local model configuration of the model that is expected in the update phase.
pub struct LocalModelConfig {
    /// The expected data type of the local model.
    // In the current state it is not possible to configure a coordinator in which
    // the scalar data type and the model data type are different. Therefore, we assume here
    // that the scalar data type is the same as the model data type.
    pub data_type: DataType,
    /// The expected length of the local model.
    pub len: usize,
}

#[derive(Error, Debug)]
#[error("failed to send a PET message")]
pub struct SendMessageError;

/// Round freshness indicator
pub enum RoundFreshness {
    /// A new round started. The current round is outdated
    Outdated,
    /// We were not able to check whether a new round started
    Unknown,
    /// The current round is still going
    Fresh,
}

/// A serializable representation of a phase state.
///
/// We cannot serialize the state directly, even though it implements `Serialize`, because deserializing it would require knowing its type in advance:
///
/// ```ignore
/// // `buf` is a Vec<u8> that contains a serialized state that we want to deserialize
/// let state: State<???> = State::deserialize(&buf[..]).unwrap();
/// ```
#[derive(Serialize, Deserialize, From, Debug)]
pub enum SerializableState {
    NewRound(State<NewRound>),
    Awaiting(State<Awaiting>),
    Sum(State<Sum>),
    Update(State<Update>),
    Sum2(State<Sum2>),
    Sending(State<Sending>),
}

impl<P> Into<SerializableState> for Phase<P>
where
    State<P>: Into<SerializableState>,
{
    fn into(self) -> SerializableState {
        self.state.into()
    }
}
