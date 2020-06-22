use thiserror::Error;

use crate::{
    coordinator::{ProtocolEvent, RoundParameters},
    mask::model::Model,
    service::handle::{SerializedSeedDict, SerializedSumDict},
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
    /// Parameters of the current round, serialized.
    pub round_params_data_serialized: Option<Arc<Vec<u8>>>,
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
    /// Return the current sum dictionary if available. The availability depends
    /// on the current coordinator state.
    pub fn sum_dict(&self) -> Option<SerializedSumDict> {
        if let PhaseData::Update(data) = self {
            Some(data.serialized_sum_dict.clone())
        } else {
            None
        }
    }

    /// Return the current model scalar if available. The availability depends
    /// on the current coordinator state.
    pub fn scalar(&self) -> Option<f64> {
        if let PhaseData::Update(data) = self {
            Some(data.scalar)
        } else {
            None
        }
    }

    /// Return the current seed dictionary if available. The availability
    /// depends on the current coordinator state.
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
                if let Some(round_parameters_data) = self.round_parameters_data.take() {
                    // Round > 1
                    // Update the round parameters. Keep the global model from the previous round.
                    self.round_parameters_data = Some(Arc::new(
                        round_parameters_data.update_round_parameters(round_parameters.clone()),
                    ));

                    let data_unser =
                        round_parameters_data.update_round_parameters(round_parameters);
                    let data_ser = bincode::serialize(&data_unser)
                        .map_err(|e| DataUpdateError::SerializeGlobalModel(e.to_string()))?;
                    self.round_params_data_serialized = Some(Arc::new(data_ser))
                } else {
                    // Round = 1
                    // First round, update the round parameters and set the global model to None.
                    self.round_parameters_data = Some(Arc::new(RoundParametersData::from(
                        round_parameters.clone(),
                    )));

                    let data_unser = RoundParametersData::from(round_parameters);
                    let data_ser = bincode::serialize(&data_unser)
                        .map_err(|e| DataUpdateError::SerializeGlobalModel(e.to_string()))?;
                    self.round_params_data_serialized = Some(Arc::new(data_ser))
                };

                self.phase_data = Some(SumData.into());
            }
            ProtocolEvent::StartUpdate(sum_dict, scalar) => {
                let serialized_sum_dict = bincode::serialize(&sum_dict)
                    .map_err(|e| DataUpdateError::SerializeSumDict(e.to_string()))?;
                let update_data = UpdateData {
                    serialized_sum_dict: Arc::new(serialized_sum_dict),
                    scalar,
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
                        // This case should not be possible.
                        // The coordinator cannot get to the step in which the event
                        // [`End Round`] is emitted, without having any round parameters.
                        panic!("A round was completed without having any round parameters.")
                    };

                self.phase_data = None;
            }
        }
        Ok(())
    }

    pub fn round_parameters(&self) -> Option<Arc<Vec<u8>>> {
        self.round_params_data_serialized.clone()
    }

    pub fn sum_dict(&self) -> Option<SerializedSumDict> {
        self.phase_data.as_ref()?.sum_dict()
    }

    pub fn scalar(&self) -> Option<f64> {
        self.phase_data.as_ref()?.scalar()
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
    /// The scalar to weight the update participants models.
    pub scalar: f64,
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

#[derive(Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct RoundParametersData {
    /// The round parameters of the current round.
    pub round_parameters: Option<RoundParameters>,

    /// The global model of the previous round.
    pub global_model: Option<Model>,
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
        Ok(RoundParametersData {
            global_model: Some(global_model),
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

impl From<Option<Model>> for RoundParametersData {
    fn from(global_model: Option<Model>) -> RoundParametersData {
        RoundParametersData {
            global_model,
            ..Default::default()
        }
    }
}
