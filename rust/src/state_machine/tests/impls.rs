use crate::{
    mask::MaskObject,
    message::{MessageOwned, Payload, Sum2Owned, SumOwned, UpdateOwned},
    state_machine::{
        events::{DictionaryUpdate, MaskLengthUpdate},
        phases::{self, PhaseState},
        requests::{
            Request,
            RequestSender,
            Sum2Request,
            Sum2Response,
            SumRequest,
            SumResponse,
            UpdateRequest,
            UpdateResponse,
        },
        StateMachine,
    },
    LocalSeedDict,
    SumParticipantEphemeralPublicKey,
    SumParticipantPublicKey,
};

use tokio::sync::oneshot;

impl<T> StateMachine<T> {
    pub fn is_update(&self) -> bool {
        match self {
            StateMachine::Update(_) => true,
            _ => false,
        }
    }

    pub fn into_update_phase_state(self) -> PhaseState<T, phases::Update> {
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

    pub fn into_sum_phase_state(self) -> PhaseState<T, phases::Sum> {
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

    pub fn into_sum2_phase_state(self) -> PhaseState<T, phases::Sum2> {
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

    pub fn into_idle_phase_state(self) -> PhaseState<T, phases::Idle> {
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

    pub fn into_unmask_phase_state(self) -> PhaseState<T, phases::Unmask> {
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

    pub fn into_error_phase_state(self) -> PhaseState<T, phases::StateError> {
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

    pub fn into_shutdown_phase_state(self) -> PhaseState<T, phases::Shutdown> {
        match self {
            StateMachine::Shutdown(state) => state,
            _ => panic!("not in shutdown state"),
        }
    }
}

impl RequestSender<Request> {
    pub async fn sum(&mut self, msg: &MessageOwned) -> SumResponse {
        let (resp_tx, resp_rx) = oneshot::channel::<SumResponse>();
        let req = Request::Sum((msg.into(), resp_tx));
        self.send(req).unwrap();
        resp_rx.await.unwrap()
    }

    pub async fn update(&mut self, msg: &MessageOwned) -> UpdateResponse {
        let (resp_tx, resp_rx) = oneshot::channel::<UpdateResponse>();
        let req = Request::Update((msg.into(), resp_tx));
        self.send(req).unwrap();
        resp_rx.await.unwrap()
    }

    pub async fn sum2(&mut self, msg: &MessageOwned) -> Sum2Response {
        let (resp_tx, resp_rx) = oneshot::channel::<Sum2Response>();
        let req = Request::Sum2((msg.into(), resp_tx));
        self.send(req).unwrap();
        resp_rx.await.unwrap()
    }
}

impl<'a> From<&'a MessageOwned> for SumRequest {
    fn from(msg: &'a MessageOwned) -> SumRequest {
        SumRequest {
            participant_pk: msg.participant_pk(),
            ephm_pk: msg.ephm_pk(),
        }
    }
}

impl<'a> From<&'a MessageOwned> for UpdateRequest {
    fn from(msg: &'a MessageOwned) -> UpdateRequest {
        UpdateRequest {
            participant_pk: msg.participant_pk(),
            local_seed_dict: msg.local_seed_dict(),
            masked_model: msg.masked_model(),
        }
    }
}

impl<'a> From<&'a MessageOwned> for Sum2Request {
    fn from(msg: &'a MessageOwned) -> Sum2Request {
        Sum2Request {
            participant_pk: msg.participant_pk(),
            mask: msg.mask(),
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

    /// Extract the mask from a sum2 message
    ///
    /// # Panic
    ///
    /// Panic if this message is not a sum2 message
    pub fn mask(&self) -> MaskObject {
        if let Payload::Sum2(Sum2Owned { mask, .. }) = &self.payload {
            mask.clone()
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
