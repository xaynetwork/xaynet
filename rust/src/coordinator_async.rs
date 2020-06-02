use std::{
    cmp::Ordering,
    collections::{HashMap, VecDeque},
    default::Default,
    mem,
    sync::Arc,
};

use sodiumoxide::{
    self,
    crypto::{box_, hash::sha256},
    randombytes::randombytes,
};
use thiserror::Error;

use crate::{
    coordinator::{MaskDict, ProtocolEvent, RoundSeed},
    crypto::{generate_encrypt_key_pair, ByteObject, SigningKeySeed},
    mask::{
        Aggregation,
        BoundType,
        DataType,
        GroupType,
        MaskConfig,
        MaskObject,
        Model,
        ModelType,
        UnmaskingError,
    },
    message::{
        Message,
        MessageOpen,
        MessageOwned,
        PayloadOwned,
        Sum2Owned,
        SumOwned,
        Tag,
        UpdateOwned,
    },
    message_processing::{MessageSink, MessageValidator, SumValidationData},
    CoordinatorPublicKey,
    CoordinatorSecretKey,
    InitError,
    LocalSeedDict,
    ParticipantPublicKey,
    ParticipantTaskSignature,
    PetError,
    SeedDict,
    SumDict,
    SumParticipantPublicKey,
    UpdateParticipantPublicKey,
};
use tokio::{
    stream::StreamExt,
    sync::{
        broadcast,
        mpsc,
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        Semaphore,
    },
    task::JoinHandle,
    time::Duration,
};

#[derive(Debug)]
pub struct Start;
#[derive(Debug)]
pub struct Idle;
#[derive(Debug)]
pub struct Sum;
#[derive(Debug)]
pub struct Update {
    sum_dict: Option<Arc<SumDict>>,
}
#[derive(Debug)]
pub struct Sum2;

// error state

#[derive(Debug)]
pub struct State<S> {
    _inner: S,
    // coordinator state
    coordinator_state: CoordinatorState,
    // message rx
    message_rx: tokio::sync::mpsc::UnboundedReceiver<()>,
}

// Functions that are available for all states
impl<S> State<S> {
    fn message_opener(&self) -> MessageOpen<'_, '_> {
        MessageOpen {
            recipient_pk: &self.coordinator_state.pk,
            recipient_sk: &self.coordinator_state.sk,
        }
    }

    fn message_open(&self, message: Vec<u8>) -> Result<MessageOwned, PetError> {
        self.message_opener()
            .open(&message)
            .map_err(|_| PetError::InvalidMessage)
    }

    async fn next_message(&mut self) -> Result<MessageOwned, PetError> {
        let message = match self.message_rx.recv().await {
            Some(message) => message,
            None => panic!("all message senders dropped!"),
        };
        self.message_open(vec![1, 2, 34]) // dummy value
    }
}

impl State<Start> {
    pub fn new() -> Result<(tokio::sync::mpsc::UnboundedSender<()>, State<Start>), InitError> {
        // crucial: init must be called before anything else in this module
        sodiumoxide::init().or(Err(InitError))?;

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<()>();

        Ok((
            tx,
            State {
                _inner: Start,
                coordinator_state: CoordinatorState {
                    seed: RoundSeed::generate(),
                    ..Default::default()
                },
                message_rx: rx,
            },
        ))
    }

    pub async fn next(self) -> State<Sum> {
        State {
            _inner: Sum,
            coordinator_state: self.coordinator_state,
            message_rx: self.message_rx,
        }
    }
}

impl State<Sum> {
    pub async fn next(mut self) -> State<Update> {
        match self.run().await {
            Ok(sum_dict) => State {
                _inner: Update {
                    sum_dict: Some(Arc::new(sum_dict)),
                },
                coordinator_state: self.coordinator_state,
                message_rx: self.message_rx,
            },
            Err(_) => {
                // error state
                panic!("")
            }
        }
    }

    async fn run(&mut self) -> Result<SumDict, PetError> {
        let mut phase_timeout = tokio::time::delay_for(tokio::time::Duration::from_millis(10000));
        let (notify_cancel, _) = broadcast::channel::<()>(1);
        let (_cancel_complete_tx, mut cancel_complete_rx) = mpsc::channel::<()>(1);
        let (sink_tx, sink) = MessageSink::new(12, Duration::from_secs(100));

        let sum_validation_data = Arc::new(SumValidationData {
            seed: self.coordinator_state.seed.clone(),
            sum: self.coordinator_state.sum,
        });

        let phase_result = tokio::select! {
            stream_result = async {
                loop {
                    let message = self.next_message().await?;

                    let participant_pk = message.header.participant_pk;
                    let sum_message = match message.payload {
                        PayloadOwned::Sum(msg) => msg,
                        _ => return Err(PetError::InvalidMessage),
                    };

                    let message_validator = MessageValidator::new(sink_tx.clone(), _cancel_complete_tx.clone(), notify_cancel.subscribe());
                    let handle_fut = message_validator.handle_message(sum_validation_data.clone(), participant_pk, sum_message);
                    tokio::spawn(async move { handle_fut.await });
                };
            } => {
                error!("something went wrong!");
                stream_result
            }
            sink_result = sink.collect() => {
                sink_result
            }
            _ = &mut phase_timeout => {
                error!("phase timed out");
                Err::<(), PetError>(PetError::InvalidMessage)
            }
        };

        // Drop the notify_cancel sender. By dropping the sender, all receivers will receive a
        // RecvError.
        drop(notify_cancel);

        // Wait until all MessageValidator tasks has been resolved/canceled.
        // (After all senders of this channel are dropped, which mean that all
        // MessageValidator has been dropped, the receiver of this channel will receive None).
        drop(_cancel_complete_tx);
        let _ = cancel_complete_rx.recv().await;

        // Return in case of an error
        phase_result?;
        // otherwise fetch and return the sum_dict
        Ok(HashMap::new())
    }
}

impl State<Update> {
    pub async fn next(self) -> State<Sum2> {
        State {
            _inner: Sum2 {},
            coordinator_state: self.coordinator_state,
            message_rx: self.message_rx,
        }
    }
}

impl State<Sum2> {
    pub async fn next(self) -> State<Idle> {
        State {
            _inner: Idle {},
            coordinator_state: self.coordinator_state,
            message_rx: self.message_rx,
        }
    }
}

impl State<Idle> {
    pub async fn next(self) -> State<Sum> {
        State {
            _inner: Sum {},
            coordinator_state: self.coordinator_state,
            message_rx: self.message_rx,
        }
    }
}

#[derive(Debug)]
pub struct CoordinatorState {
    pk: CoordinatorPublicKey, // 32 bytes
    sk: CoordinatorSecretKey, // 32 bytes

    // round parameters
    sum: f64,
    update: f64,
    seed: RoundSeed,
    min_sum: usize,
    min_update: usize,

    // round dictionaries
    /// Dictionary built during the sum phase.
    sum_dict: SumDict,
    /// Dictionary built during the update phase.
    seed_dict: SeedDict,
    /// Dictionary built during the sum2 phase.
    mask_dict: MaskDict,

    /// The masking configuration
    mask_config: MaskConfig,

    /// The aggregated masked model being built in the current round.
    aggregation: Aggregation,

    /// Events emitted by the state machine
    events: VecDeque<ProtocolEvent>,
}

impl Default for CoordinatorState {
    fn default() -> Self {
        let pk = CoordinatorPublicKey::zeroed();
        let sk = CoordinatorSecretKey::zeroed();
        let sum = 0.01_f64;
        let update = 0.1_f64;
        let seed = RoundSeed::zeroed();
        let min_sum = 1_usize;
        let min_update = 3_usize;
        let sum_dict = SumDict::new();
        let seed_dict = SeedDict::new();
        let mask_dict = MaskDict::new();
        let events = VecDeque::new();
        let mask_config = MaskConfig {
            group_type: GroupType::Prime,
            data_type: DataType::F32,
            bound_type: BoundType::B0,
            model_type: ModelType::M3,
        };
        let aggregation = Aggregation::new(mask_config);
        Self {
            pk,
            sk,
            sum,
            update,
            seed,
            min_sum,
            min_update,
            sum_dict,
            seed_dict,
            mask_dict,
            events,
            mask_config,
            aggregation,
        }
    }
}
