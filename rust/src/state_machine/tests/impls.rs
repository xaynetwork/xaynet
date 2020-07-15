use crate::{
    mask::MaskObject,
    state_machine::{
        phases::{self, PhaseState},
        requests::{
            Request,
            RequestSender,
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
    UpdateParticipantPublicKey,
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

// FIXME: this is very convenient for the tests but it could actually
// be used in the codebase. The only problem right now is that if we
// need to rethink error handling in order to avoid nested Result. For
// tests unwrapping is fine though.
#[cfg(test)]
impl RequestSender<Request> {
    pub async fn sum(
        &mut self,
        participant_pk: SumParticipantPublicKey,
        ephm_pk: SumParticipantEphemeralPublicKey,
    ) -> SumResponse {
        let (resp_tx, resp_rx) = oneshot::channel::<SumResponse>();
        let req = Request::Sum((
            SumRequest {
                participant_pk,
                ephm_pk,
            },
            resp_tx,
        ));
        self.send(req).unwrap();
        resp_rx.await.unwrap()
    }

    pub async fn update(
        &mut self,
        participant_pk: UpdateParticipantPublicKey,
        local_seed_dict: LocalSeedDict,
        masked_model: MaskObject,
    ) -> UpdateResponse {
        let (resp_tx, resp_rx) = oneshot::channel::<UpdateResponse>();
        let req = Request::Update((
            UpdateRequest {
                participant_pk,
                local_seed_dict,
                masked_model,
            },
            resp_tx,
        ));
        self.send(req).unwrap();
        resp_rx.await.unwrap()
    }
}
