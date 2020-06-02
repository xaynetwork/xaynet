use std::sync::Arc;
use tokio::{
    sync::{
        broadcast,
        mpsc::{unbounded_channel, Sender, UnboundedReceiver, UnboundedSender},
    },
    time::Duration,
};

use crate::{
    coordinator::RoundSeed,
    crypto::ByteObject,
    message::SumOwned,
    ParticipantPublicKey,
    ParticipantTaskSignature,
    PetError,
    SumParticipantPublicKey,
};

// A sink to collect the results of the MessageValidator tasks.
pub struct MessageSink {
    // The minimum number of successfully validated messages.
    min_messages: usize,
    // A counter that is incremented when a message has been successfully validated.
    successful_messages: usize,
    // The channel receiver that receives the results of the MessageValidator tasks.
    sink_rx: UnboundedReceiver<Result<(), PetError>>,
    // The minimum duration to wait.
    min_duration: Duration,
}

impl MessageSink {
    pub fn new(
        min_messages: usize,
        min_duration: Duration,
    ) -> (UnboundedSender<Result<(), PetError>>, Self) {
        let (success_tx, sink_rx) = unbounded_channel();
        (
            success_tx,
            Self {
                min_messages,
                successful_messages: 0,
                sink_rx,
                min_duration,
            },
        )
    }

    pub async fn collect(self) -> Result<(), PetError> {
        let MessageSink {
            min_messages,
            mut successful_messages,
            mut sink_rx,
            min_duration,
        } = self;

        // Collect the results of the MessageValidator tasks. The collect future will be
        // successfully resolved when the minimum duration has been waited and when the minimum
        // number of successful validated messages has been reached.
        // The first failed MessageValidator result causes the collection to be canceled. In this
        // case the collect future will be resolved with an error.
        let wait_min_duration = async move {
            tokio::time::delay_for(min_duration).await;
            Ok::<(), PetError>(())
        };

        let collection = async move {
            loop {
                let _message = sink_rx
                    .recv()
                    .await
                    // First '?' operator is applied on the result of '.recv()', the second one on
                    // the message itself (which has the type 'Result<(), PetError>').
                    .ok_or(PetError::InvalidMessage)??;

                successful_messages += 1;

                if successful_messages == min_messages {
                    break Ok::<(), PetError>(());
                }
            }
        };

        tokio::try_join!(wait_min_duration, collection).map(|_| ())
    }
}

pub struct MessageValidator {
    // A sender through which the result of the massage validation is sent.
    sink_tx: UnboundedSender<Result<(), PetError>>,
    // We will never send anything on this channel.
    // We use this channel to keep track of which MessageValidator tasks are still alive.
    _cancel_complete_tx: Sender<()>,
    // The channel receiver that receives the cancel notification.
    notify_cancel: broadcast::Receiver<()>,
}

impl MessageValidator {
    pub fn new(
        sink_tx: UnboundedSender<Result<(), PetError>>,
        cancel_complete_tx: Sender<()>,
        notify_cancel: broadcast::Receiver<()>,
    ) -> Self {
        Self {
            sink_tx,
            _cancel_complete_tx: cancel_complete_tx,
            notify_cancel,
        }
    }

    /// Validate and handle a sum message.
    pub async fn handle_message(
        self,
        coordinator_data: Arc<SumValidationData>,
        pk: ParticipantPublicKey,
        message: SumOwned,
    ) {
        // Extract all fields of the MessageValidator struct. This is necessary to bypass borrow
        // issues in the tokio::select macro.
        // It is imported to extract _cancel_complete_tx as well because otherwise the channel
        // will be dropped too early.
        let MessageValidator {
            sink_tx,
            _cancel_complete_tx,
            mut notify_cancel,
        } = self;

        tokio::select! {
            result = async {
                MessageValidator::validate_sum_task(&coordinator_data, &pk, &message.sum_signature).await
                // async call to Redis
                // self.coordinator_state.sum_dict.insert(pk, message.ephm_pk);
            } => {let _ = sink_tx.send(result);}
            _ = notify_cancel.recv() => {info!("drop message validation future")}
        };

        // _cancel_complete_tx is dropped
    }

    /// Validate a sum signature and its implied task.
    async fn validate_sum_task(
        coordinator_state: &Arc<SumValidationData>,
        pk: &SumParticipantPublicKey,
        sum_signature: &ParticipantTaskSignature,
    ) -> Result<(), PetError> {
        println!("validate_sum_task");
        // Ok(())
        if pk.verify_detached(
            sum_signature,
            &[coordinator_state.seed.as_slice(), b"sum"].concat(),
        ) && sum_signature.is_eligible(coordinator_state.sum)
        {
            Ok(())
        } else {
            Err(PetError::InvalidMessage)
        }
    }
}

pub struct SumValidationData {
    pub sum: f64,
    pub seed: RoundSeed,
}
