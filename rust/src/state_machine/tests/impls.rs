use crate::{
    mask::MaskObject,
    message::{MessageOwned, Payload, Sum2Owned, SumOwned, UpdateOwned},
    state_machine::{
        events::{DictionaryUpdate, MaskLengthUpdate},
        phases::{self, PhaseState},
        requests::RequestSender,
        StateMachine,
        StateMachineResult,
    },
    utils::Request,
    LocalSeedDict,
    SumParticipantEphemeralPublicKey,
    SumParticipantPublicKey,
};

impl RequestSender {
    pub async fn msg(&self, msg: &MessageOwned) -> StateMachineResult {
        self.request(Request::new(msg.clone())).await
    }
}

impl StateMachine {
    pub fn is_update(&self) -> bool {
        match self {
            StateMachine::Update(_) => true,
            _ => false,
        }
    }

    pub fn into_update_phase_state(self) -> PhaseState<phases::Update> {
        match self {
            StateMachine::Update(state) => state,
            _ => panic!("not in update state"),
        }
    }

    pub fn is_sum(&self) -> bool {
        match self {
            StateMachine::Sum(_) => true,
            _ => false,
        }
    }

    pub fn into_sum_phase_state(self) -> PhaseState<phases::Sum> {
        match self {
            StateMachine::Sum(state) => state,
            _ => panic!("not in sum state"),
        }
    }

    pub fn is_sum2(&self) -> bool {
        match self {
            StateMachine::Sum2(_) => true,
            _ => false,
        }
    }

    pub fn into_sum2_phase_state(self) -> PhaseState<phases::Sum2> {
        match self {
            StateMachine::Sum2(state) => state,
            _ => panic!("not in sum2 state"),
        }
    }

    pub fn is_idle(&self) -> bool {
        match self {
            StateMachine::Idle(_) => true,
            _ => false,
        }
    }

    pub fn into_idle_phase_state(self) -> PhaseState<phases::Idle> {
        match self {
            StateMachine::Idle(state) => state,
            _ => panic!("not in idle state"),
        }
    }

    pub fn is_unmask(&self) -> bool {
        match self {
            StateMachine::Unmask(_) => true,
            _ => false,
        }
    }

    pub fn into_unmask_phase_state(self) -> PhaseState<phases::Unmask> {
        match self {
            StateMachine::Unmask(state) => state,
            _ => panic!("not in unmask state"),
        }
    }

    pub fn is_error(&self) -> bool {
        match self {
            StateMachine::Error(_) => true,
            _ => false,
        }
    }

    pub fn into_error_phase_state(self) -> PhaseState<phases::StateError> {
        match self {
            StateMachine::Error(state) => state,
            _ => panic!("not in error state"),
        }
    }

    pub fn is_shutdown(&self) -> bool {
        match self {
            StateMachine::Shutdown(_) => true,
            _ => false,
        }
    }

    pub fn into_shutdown_phase_state(self) -> PhaseState<phases::Shutdown> {
        match self {
            StateMachine::Shutdown(state) => state,
            _ => panic!("not in shutdown state"),
        }
    }
}

impl MessageOwned {
    /// Extract the participant public key from the message.
    pub fn participant_pk(&self) -> SumParticipantPublicKey {
        self.header.participant_pk
    }

    /// Extract the ephemeral public key from a sum message.
    ///
    /// # Panic
    ///
    /// Panic if this message is not a sum message
    pub fn ephm_pk(&self) -> SumParticipantEphemeralPublicKey {
        if let Payload::Sum(SumOwned { ephm_pk, .. }) = &self.payload {
            *ephm_pk
        } else {
            panic!("not a sum message");
        }
    }

    /// Extract the masked model from an update message
    ///
    /// # Panic
    ///
    /// Panic if this message is not an update message
    pub fn masked_model(&self) -> MaskObject {
        if let Payload::Update(UpdateOwned { masked_model, .. }) = &self.payload {
            masked_model.clone()
        } else {
            panic!("not an update message");
        }
    }

    /// Extract the masked scalar from an update message
    ///
    /// # Panic
    ///
    /// Panic if this message is not an update message
    pub fn masked_scalar(&self) -> MaskObject {
        if let Payload::Update(UpdateOwned { masked_scalar, .. }) = &self.payload {
            masked_scalar.clone()
        } else {
            panic!("not an update message");
        }
    }

    /// Extract the local seed dictioanry from an update message
    ///
    /// # Panic
    ///
    /// Panic if this message is not an update message
    pub fn local_seed_dict(&self) -> LocalSeedDict {
        if let Payload::Update(UpdateOwned {
            local_seed_dict, ..
        }) = &self.payload
        {
            local_seed_dict.clone()
        } else {
            panic!("not an update message");
        }
    }

    /// Extract the model mask from a sum2 message
    ///
    /// # Panic
    ///
    /// Panic if this message is not a sum2 message
    pub fn mask(&self) -> MaskObject {
        if let Payload::Sum2(Sum2Owned { model_mask, .. }) = &self.payload {
            model_mask.clone()
        } else {
            panic!("not a sum2 message");
        }
    }

    /// Extract the scalar mask from a sum2 message
    ///
    /// # Panic
    ///
    /// Panic if this message is not a sum2 message
    pub fn scalar_mask(&self) -> MaskObject {
        if let Payload::Sum2(Sum2Owned { scalar_mask, .. }) = &self.payload {
            scalar_mask.clone()
        } else {
            panic!("not a sum2 message");
        }
    }
}

impl<D> DictionaryUpdate<D> {
    pub fn unwrap(self) -> std::sync::Arc<D> {
        if let DictionaryUpdate::New(inner) = self {
            inner
        } else {
            panic!("DictionaryUpdate::Invalidate");
        }
    }
}

impl MaskLengthUpdate {
    pub fn unwrap(self) -> usize {
        if let MaskLengthUpdate::New(inner) = self {
            inner
        } else {
            panic!("MaskLengthUpdate::Invalidate");
        }
    }
}
