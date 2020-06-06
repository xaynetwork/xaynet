use crate::{
    coordinator::RoundSeed,
    coordinator_async::{store::client::Connection, StateError},
    crypto::ByteObject,
    mask::MaskObject,
    message::{Sum2Owned, SumOwned, UpdateOwned},
    LocalSeedDict,
    ParticipantPublicKey,
    ParticipantTaskSignature,
    PetError,
    SumParticipantEphemeralPublicKey,
    SumParticipantPublicKey,
    UpdateParticipantPublicKey,
};
use std::{future::Future, sync::Arc};
use tokio::{
    sync::{
        broadcast,
        mpsc::{unbounded_channel, Sender, UnboundedReceiver, UnboundedSender},
    },
    time::Duration,
};

pub struct SumValidationData {
    pub sum: f64,
    pub seed: RoundSeed,
}

pub struct UpdateValidationData {
    pub sum: f64,
    pub update: f64,
    pub seed: RoundSeed,
}

// A sink to collect the results of the MessageHandler tasks.
pub struct MessageSink {
    // The minimum number of successfully validated messages.
    min_messages: usize,
    // A counter that is incremented when a message has been successfully validated.
    successful_messages: usize,
    // The channel receiver that receives the results of the MessageHandler tasks.
    sink_rx: UnboundedReceiver<Result<(), PetError>>,
    // The minimum duration to wait.
    min_duration: Duration,
    // Message collection time out.
    max_duration: Duration,
}

impl MessageSink {
    pub fn new(
        min_messages: usize,
        min_duration: Duration,
        max_duration: Duration,
    ) -> (UnboundedSender<Result<(), PetError>>, Self) {
        if max_duration < min_duration {
            panic!("max_duration must be greater than min_duration")
        };

        let (success_tx, sink_rx) = unbounded_channel();
        (
            success_tx,
            Self {
                min_messages,
                successful_messages: 0,
                sink_rx,
                min_duration,
                max_duration,
            },
        )
    }

    pub async fn collect(self) -> Result<(), StateError> {
        let MessageSink {
            min_messages,
            mut successful_messages,
            mut sink_rx,
            min_duration,
            max_duration,
        } = self;
        // Collect the results of the MessageHandler tasks. The collect future will be
        // successfully resolved when the minimum duration has been waited and when the minimum
        // number of successful validated messages has been reached.
        // The first failed MessageHandler result causes the collection to be canceled. In this
        // case the collect future will be resolved with an error.
        let min_collection_duration = async move {
            tokio::time::delay_for(min_duration).await;
            debug!("waited min collection duration");
            Ok::<(), PetError>(())
        };

        let message_collection = async move {
            loop {
                let _message = sink_rx
                    .recv()
                    .await
                    // First '?' operator is applied on the result of '.recv()', the second one on
                    // the message itself (which has the type 'Result<(), StateError>').
                    .ok_or(PetError::InvalidMessage)??;

                successful_messages += 1;

                if successful_messages == min_messages {
                    break Ok::<(), PetError>(());
                }
            }
        };

        let mut max_collection_duration = tokio::time::delay_for(max_duration);

        tokio::select! {
            collection_result = async {
                tokio::try_join!(min_collection_duration, message_collection).map(|_| ())
            }=> {
                collection_result.map_err(From::from)
            }
            _ = &mut max_collection_duration => {
                Err::<(), StateError>(StateError::Timeout)
            }
        }
    }
}

pub struct MessageHandler {
    // A sender through which the result of the massage validation is sent.
    sink_tx: UnboundedSender<Result<(), PetError>>,
    // We will never send anything on this channel.
    // We use this channel to keep track of which MessageHandler tasks are still alive.
    _cancel_complete_tx: Sender<()>,
    // The channel receiver that receives the cancel notification.
    notify_cancel: broadcast::Receiver<()>,
}

impl MessageHandler {
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

    async fn handle_message(
        mut self,
        message_validation_fut: impl Future<Output = Result<(), PetError>>,
    ) {
        tokio::select! {
            result = message_validation_fut => {let _ = self.sink_tx.send(result);}
            _ = self.notify_cancel.recv() => {println!("drop message validation future")}
        };

        // _cancel_complete_tx is dropped
    }
}

// Sum message validator
impl MessageHandler {
    /// Validate and handle a sum message.
    pub async fn handle_sum_message(
        self,
        coordinator_state: Arc<SumValidationData>,
        pk: ParticipantPublicKey,
        message: SumOwned,
        redis: Connection,
    ) {
        let message_validation_fut = async {
            Self::validate_sum_task(&coordinator_state, &pk, &message.sum_signature)?;
            Self::add_sum_participant(&pk, &message.ephm_pk, redis).await
        };
        self.handle_message(message_validation_fut).await;
    }

    /// Validate a sum signature and its implied task.
    fn validate_sum_task(
        coordinator_state: &Arc<SumValidationData>,
        pk: &SumParticipantPublicKey,
        sum_signature: &ParticipantTaskSignature,
    ) -> Result<(), PetError> {
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

    async fn add_sum_participant(
        pk: &SumParticipantPublicKey,
        ephm_pk: &SumParticipantEphemeralPublicKey,
        redis: Connection,
    ) -> Result<(), PetError> {
        match redis.add_sum_participant(*pk, *ephm_pk).await {
            // key is new
            Ok(1) => Ok(()),
            // key already exists or redis returned an error
            Ok(_) | Err(_) => Err(PetError::InvalidMessage),
        }
    }
}

// Update message validator
impl MessageHandler {
    /// Validate and handle an update message.
    pub async fn handle_update_message(
        self,
        coordinator_state: Arc<UpdateValidationData>,
        pk: ParticipantPublicKey,
        message: UpdateOwned,
        redis: Connection,
    ) {
        let message_validation_fut = async {
            let UpdateOwned {
                sum_signature,
                update_signature,
                local_seed_dict,
                masked_model,
            } = message;
            Self::validate_update_task(&coordinator_state, &pk, &sum_signature, &update_signature)?;
            // Try to update local seed dict first. If this fail, we do
            // not want to aggregate the model.

            Self::add_local_seed_dict(&pk, &local_seed_dict, redis).await

            // Check if aggregation can be performed, and do it.
            //
            // self.aggregation
            //     .validate_aggregation(&masked_model)
            //     .map_err(|_| PetError::InvalidMessage)?;
            // self.aggregation.aggregate(masked_model);
        };
        self.handle_message(message_validation_fut).await;
    }

    /// Validate an update signature and its implied task.
    fn validate_update_task(
        coordinator_state: &Arc<UpdateValidationData>,
        pk: &UpdateParticipantPublicKey,
        sum_signature: &ParticipantTaskSignature,
        update_signature: &ParticipantTaskSignature,
    ) -> Result<(), PetError> {
        if pk.verify_detached(
            sum_signature,
            &[coordinator_state.seed.as_slice(), b"sum"].concat(),
        ) && pk.verify_detached(
            update_signature,
            &[coordinator_state.seed.as_slice(), b"update"].concat(),
        ) && !sum_signature.is_eligible(coordinator_state.sum)
            && update_signature.is_eligible(coordinator_state.update)
        {
            Ok(())
        } else {
            Err(PetError::InvalidMessage)
        }
    }

    /// Add a local seed dictionary to the seed dictionary. Fails if it contains invalid keys or it
    /// is a repetition.
    async fn add_local_seed_dict(
        pk: &UpdateParticipantPublicKey,
        local_seed_dict: &LocalSeedDict,
        redis: Connection,
    ) -> Result<(), PetError> {
        // Should we perform the checks in add_local_seed_dict in the coordinator or
        // in redis?
        // if local_seed_dict.keys().len() == sum_cache.len()
        //     && local_seed_dict
        //         .keys()
        //         .all(|pk| sum_cache.sum_pks().contains(pk))
        // {
        redis
            .update_seed_dict(*pk, &local_seed_dict)
            .await
            .map_err(|_| PetError::InvalidMessage)
        // } else {
        //     Err(PetError::InvalidMessage)
        // }
    }
}

// Sum2 message validator
impl MessageHandler {
    /// Validate and handle an update message.
    pub async fn handle_sum2_message(
        self,
        coordinator_state: Arc<SumValidationData>,
        pk: ParticipantPublicKey,
        message: Sum2Owned,
        redis: Connection,
    ) {
        let message_validation_fut = async {
            // We move the participant key here to make sure a participant
            // cannot submit a mask multiple times
            // if self.sum_dict.remove(pk).is_none() {
            //     return Err(PetError::InvalidMessage);
            // }

            Self::validate_sum_task(&coordinator_state, &pk, &message.sum_signature)?;
            Self::add_mask(&pk, &message.mask, redis).await
        };
        self.handle_message(message_validation_fut).await;
    }

    /// Add a mask to the mask dictionary. Fails if the sum participant didn't register in the sum
    /// phase or it is a repetition.
    async fn add_mask(
        pk: &SumParticipantPublicKey,
        mask: &MaskObject,
        redis: Connection,
    ) -> Result<(), PetError> {
        // match redis
        //     .remove_sum_dict_entry(*pk)
        //     .await
        // {
        //     // field was deleted
        //     Ok(1) => (),
        //     // field does not exist or redis err
        //     Ok(_) | Err(_) => return Err(PetError::InvalidMessage),
        // }
        // (sum_dict, updates, seed_dict)

        redis
            .incr_mask_count(mask)
            .await
            .map_err(|_| PetError::InvalidMessage)
    }
}
