use thiserror::Error;

use crate::{
    coordinator::{ProtocolEvent, RoundParameters},
    mask::model::Model,
    service::handle::{SerializedGlobalModel, SerializedSeedDict, SerializedSumDict},
    SeedDict,
    SumParticipantPublicKey,
};
use derive_more::From;
use std::{collections::HashMap, sync::Arc};

/// Data that the service keeps track of.
#[derive(From, Default)]
pub struct Data {
    /// Parameters of the current round.
    pub round_parameters_data: Option<Arc<RoundParametersData>>,
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
    Aggregation,
}

impl PhaseData {
    /// Return the current sum dictionary if it is available. The
    /// availability of the sum dictionary depends on the current
    /// coordinatore state.
    pub fn sum_dict(&self) -> Option<SerializedSumDict> {
        if let PhaseData::Update(data) = self {
            Some(data.serialized_sum_dict.clone())
        } else {
            None
        }
    }

    /// Return the current seed dictionary if it is available. The
    /// availability of the seed dictionary depends on the current
    /// coordinatore state.
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
    #[error("failed to serialize the global model: {0}")]
    SerializeGlobalModel(String),
}

impl Data {
    pub fn new() -> Self {
        Data::default()
    }

    /// Handle the given event and update the state accordingly
    pub fn update(&mut self, event: ProtocolEvent) -> Result<(), DataUpdateError> {
        match event {
            ProtocolEvent::StartSum(round_parameters) => {
                self.round_parameters_data =
                    if let Some(round_parameters_data) = self.round_parameters_data.take() {
                        // Round > 1
                        // Update the round parameters. Keep the global model from the previous round.
                        Some(Arc::new(
                            round_parameters_data.update_round_parameters(round_parameters),
                        ))
                    } else {
                        // Round = 1
                        // First round, update the round parameters and set the global model to None.
                        Some(Arc::new(RoundParametersData::from(round_parameters)))
                    };

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
            ProtocolEvent::EndRound(global_model) => {
                // The coordinator has ended the round but hasn't yet started a new one.
                // Therefore, we can only publish the global model and set all other round
                // parameters to None because the round is already over.

                self.round_parameters_data =
                    if let Some(round_parameters_data) = self.round_parameters_data.take() {
                        // Update the global model and set all other round parameters to None.
                        if let Some(global_model) = global_model {
                            Some(Arc::new(
                                round_parameters_data.update_global_model(global_model)?,
                            ))
                        } else {
                            // Something went wrong. Keep the current global model for a new round
                            // but set all other round parameters to None.
                            Some(Arc::new(RoundParametersData::from(
                                round_parameters_data.global_model.clone(),
                            )))
                        }
                    } else {
                        // Normally that case should not be possible.
                        None
                    };

                self.phase_data = None;
            }
        }
        Ok(())
    }

    pub fn round_parameters(&self) -> Option<Arc<RoundParametersData>> {
        self.round_parameters_data.clone()
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

#[derive(Debug, PartialEq, Default)]
pub struct RoundParametersData {
    /// The round parameters of the current round.
    pub round_parameters: Option<RoundParameters>,

    /// The global model of the previous round.
    pub global_model: Option<SerializedGlobalModel>,
}

impl RoundParametersData {
    /// Update the round parameters. Keep the global model from the previous round.
    /// If it is the first round, the value of the global model will be None.
    fn update_round_parameters(&self, round_parameters: RoundParameters) -> RoundParametersData {
        RoundParametersData {
            round_parameters: Some(round_parameters),
            global_model: self.global_model.clone(),
        }
    }

    /// Update the global model. Set all other round parameters to None.
    fn update_global_model(
        &self,
        global_model: Model,
    ) -> Result<RoundParametersData, DataUpdateError> {
        let serialized = bincode::serialize(&global_model)
            .map_err(|e| DataUpdateError::SerializeGlobalModel(e.to_string()))?;
        Ok(RoundParametersData {
            global_model: Some(Arc::new(serialized)),
            ..Default::default()
        })
    }
}

impl From<RoundParameters> for RoundParametersData {
    fn from(round_parameters: RoundParameters) -> RoundParametersData {
        RoundParametersData {
            round_parameters: Some(round_parameters),
            ..Default::default()
        }
    }
}

impl From<Option<SerializedGlobalModel>> for RoundParametersData {
    fn from(serialized_global_model: Option<SerializedGlobalModel>) -> RoundParametersData {
        RoundParametersData {
            global_model: serialized_global_model,
            ..Default::default()
        }
    }
}
