use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    coordinator::{Coordinator, ProtocolEvent, RoundParameters},
    service::handle::{SerializedSeedDict, SerializedSumDict},
    MaskHash,
    SeedDict,
    SumDict,
    SumParticipantPublicKey,
};
use derive_more::From;
use sodiumoxide::crypto::box_;
use std::{collections::HashMap, sync::Arc};

/// Data that the service keeps track of.
#[derive(From, Default)]
pub struct Data {
    /// Parameters of the current round. If there is no round in
    /// progress, this is `None`.
    pub round_parameters: Option<Arc<RoundParameters>>,
    /// Data relevant to the current phase of the protocol. During the
    /// update phase, this contains the sum dictionary to be sent to
    /// the update participants for instance, while during the sum2
    /// phase it contains the seed dictionaries.
    pub phase_data: Option<PhaseData>,
}

/// Data held by the service in specific phases
#[derive(From)]
pub enum PhaseData {
    /// Data held by the service during the sum phase
    #[from]
    Sum(SumData),

    /// Data held by the service during the update phase
    #[from]
    Update(UpdateData),

    /// Data held by the service during the sum2 phase
    #[from]
    Sum2(Sum2Data),

    /// Data held by the service during the aggregation phase
    #[from]
    Aggregation(AggregationData),
}

impl PhaseData {
    pub fn sum_dict(&self) -> Option<SerializedSumDict> {
        if let PhaseData::Update(data) = self {
            Some(data.serialized_sum_dict.clone())
        } else {
            None
        }
    }

    pub fn seed_dict(
        &mut self,
        pk: SumParticipantPublicKey,
    ) -> Result<Option<SerializedSeedDict>, DataUpdateError> {
        if let PhaseData::Sum2(data) = self {
            data.get_seed_dict(pk)
        } else {
            Ok(None)
        }
    }
}

/// Error returned when the state cannot be updated.
#[derive(Debug, Error)]
pub enum DataUpdateError {
    #[error("failed to serialize the sum dictionary: {0}")]
    SerializeSumDict(String),
    #[error("failed to serialize a seed dictionary: {0}")]
    SerializeSeedDict(String),
}

impl Data {
    pub fn new() -> Self {
        Data::default()
    }

    /// Handle the given event and update the state accordingly
    pub fn update(&mut self, event: ProtocolEvent) -> Result<(), DataUpdateError> {
        match event {
            ProtocolEvent::StartSum(round_parameters) => {
                self.round_parameters = Some(Arc::new(round_parameters));
                self.phase_data = Some(SumData.into());
            }
            ProtocolEvent::StartUpdate(sum_dict) => {
                let serialized_sum_dict = bincode::serialize(&sum_dict)
                    .map_err(|e| DataUpdateError::SerializeSumDict(e.to_string()))?;
                let update_data = UpdateData {
                    serialized_sum_dict: Arc::new(serialized_sum_dict),
                };
                self.phase_data = Some(update_data.into());
            }
            ProtocolEvent::StartSum2(seed_dict) => {
                let sum2_data = Sum2Data {
                    seed_dict,
                    serialized_seed_dict: HashMap::new(),
                };
                self.phase_data = Some(sum2_data.into());
            }
            ProtocolEvent::EndRound(Some(mask_hash)) => {
                self.round_parameters = None;
                self.phase_data = Some(AggregationData { mask_hash }.into());
            }
            ProtocolEvent::EndRound(None) => {
                self.round_parameters = None;
                self.phase_data = None;
            }
        }
        Ok(())
    }

    pub fn round_parameters(&self) -> Option<Arc<RoundParameters>> {
        self.round_parameters.clone()
    }

    pub fn sum_dict(&self) -> Option<SerializedSumDict> {
        self.phase_data.as_ref()?.sum_dict()
    }

    pub fn seed_dict(
        &mut self,
        pk: SumParticipantPublicKey,
    ) -> Result<Option<SerializedSeedDict>, DataUpdateError> {
        match self.phase_data.as_mut() {
            Some(data) => data.seed_dict(pk),
            None => Ok(None),
        }
    }
}

/// Service data specific to the sum phase
pub struct SumData;

/// Service data specific to the update phase
pub struct UpdateData {
    /// The sum dictionary, already serialized so that it can direclty
    /// be sent to the clients that request it
    pub serialized_sum_dict: SerializedSeedDict,
}

/// Service data specific to the sum2 phase
pub struct Sum2Data {
    /// The seed dictionary produced by the update phase.
    pub seed_dict: SeedDict,
    /// The seed dictionary with serialized values that can directly
    /// be sent to the clients that request it.
    pub serialized_seed_dict: HashMap<SumParticipantPublicKey, SerializedSeedDict>,
}

impl Sum2Data {
    /// Retrieve a serialized seed dictionary that corresponds to the
    /// given public key.
    fn get_seed_dict(
        &mut self,
        pk: SumParticipantPublicKey,
    ) -> Result<Option<SerializedSeedDict>, DataUpdateError> {
        // If we already serialized the dictionary for the given
        // public key, just return it
        if let Some(value) = self.serialized_seed_dict.get(&pk) {
            return Ok(Some(value.clone()));
        }

        // Otherwise, check if we have a dictionary for the requested
        // public key. If so, serialize and store it, in case it is
        // requested again in the future.
        if let Some(dict) = self.seed_dict.remove(&pk) {
            // FIXME: if we have many participants these
            // serializations will have a non-negligible cost. We may
            // have to offload that.
            let serialized = bincode::serialize(&dict)
                .map_err(|e| DataUpdateError::SerializeSeedDict(e.to_string()))?;
            let value = Arc::new(serialized);
            self.serialized_seed_dict.insert(pk, value.clone());
            return Ok(Some(value));
        }

        // We don't have a seed dictionary for the given key
        Ok(None)
    }
}

/// Service data specific to the aggregation phase
pub struct AggregationData {
    pub mask_hash: MaskHash,
}
