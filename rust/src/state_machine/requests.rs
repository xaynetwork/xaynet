use crate::{
    mask::MaskObject,
    LocalSeedDict,
    ParticipantPublicKey,
    PetError,
    SumParticipantEphemeralPublicKey,
    SumParticipantPublicKey,
    UpdateParticipantPublicKey,
};
use tokio::sync::oneshot;

// placeholders
// will be removed after the tower integration
pub enum Request {
    Sum(SumRequest),
    Update(UpdateRequest),
    Sum2(Sum2Request),
}

pub struct SumRequest {
    pub participant_pk: SumParticipantPublicKey,
    pub ephm_pk: SumParticipantEphemeralPublicKey,
    pub response_tx: oneshot::Sender<Result<(), PetError>>,
}

pub struct UpdateRequest {
    pub participant_pk: UpdateParticipantPublicKey,
    pub local_seed_dict: LocalSeedDict,
    pub masked_model: MaskObject,
    pub response_tx: oneshot::Sender<Result<(), PetError>>,
}

pub struct Sum2Request {
    pub participant_pk: ParticipantPublicKey,
    pub mask: MaskObject,
    pub response_tx: oneshot::Sender<Result<(), PetError>>,
}
