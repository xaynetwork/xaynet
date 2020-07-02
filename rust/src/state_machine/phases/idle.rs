use crate::{
    crypto::{encrypt::EncryptKeyPair, sign::SigningKeySeed, ByteObject},
    state_machine::{
        coordinator::{CoordinatorState, RoundSeed},
        events::{DictionaryUpdate, MaskLengthUpdate, PhaseEvent, ScalarUpdate},
        phases::{Phase, PhaseState, Sum},
        requests::RequestReceiver,
        StateMachine,
    },
};

use sodiumoxide::crypto::hash::sha256;

#[derive(Debug)]
pub struct Idle;

#[async_trait]
impl<R> Phase<R> for PhaseState<R, Idle>
where
    R: Send,
{
    async fn next(mut self) -> Option<StateMachine<R>> {
        info!("starting idle phase");

        self.coordinator_state.round_params.id += 1;
        let round_id = self.coordinator_state.round_params.id;
        info!("incremented round id to {}", round_id);

        info!("updating the keys");
        self.gen_round_keypair();

        info!("updating round thresholds");
        self.update_round_thresholds();

        info!("updating round seeds");
        self.update_round_seed();

        let events = &mut self.coordinator_state.events;

        info!("broadcasting new keys");
        events.broadcast_keys(
            self.coordinator_state.round_params.id,
            self.coordinator_state.keys.clone(),
        );

        info!("broadcasting idle phase event");
        events.broadcast_phase(round_id, PhaseEvent::Idle);

        info!("broadcasting invalidation of sum dictionary from previous round");
        events.broadcast_sum_dict(round_id, DictionaryUpdate::Invalidate);

        info!("broadcasting invalidation of seed dictionary from previous round");
        events.broadcast_seed_dict(round_id, DictionaryUpdate::Invalidate);

        info!("broadcasting invalidation of scalar from previous round");
        events.broadcast_scalar(round_id, ScalarUpdate::Invalidate);

        info!("broadcasting invalidation of mask length from previous round");
        events.broadcast_mask_length(round_id, MaskLengthUpdate::Invalidate);

        info!("broadcasting new round parameters");
        events.broadcast_params(self.coordinator_state.round_params.clone());

        // TODO: add a delay to prolongate the idle phase

        info!("going to sum phase");
        Some(PhaseState::<R, Sum>::new(self.coordinator_state, self.request_rx).into())
    }
}

impl<R> PhaseState<R, Idle> {
    pub fn new(coordinator_state: CoordinatorState, request_rx: RequestReceiver<R>) -> Self {
        Self {
            inner: Idle,
            coordinator_state,
            request_rx,
        }
    }

    fn update_round_thresholds(&mut self) {}

    /// Update the seed round parameter.
    fn update_round_seed(&mut self) {
        // safe unwrap: `sk` and `seed` have same number of bytes
        let (_, sk) =
            SigningKeySeed::from_slice_unchecked(self.coordinator_state.keys.secret.as_slice())
                .derive_signing_key_pair();
        let signature = sk.sign_detached(
            &[
                self.coordinator_state.round_params.seed.as_slice(),
                &self.coordinator_state.round_params.sum.to_le_bytes(),
                &self.coordinator_state.round_params.update.to_le_bytes(),
            ]
            .concat(),
        );
        // Safe unwrap: the length of the hash is 32 bytes
        self.coordinator_state.round_params.seed =
            RoundSeed::from_slice_unchecked(sha256::hash(signature.as_slice()).as_ref());
    }

    /// Generate fresh round credentials.
    fn gen_round_keypair(&mut self) {
        self.coordinator_state.keys = EncryptKeyPair::generate();
        self.coordinator_state.round_params.pk = self.coordinator_state.keys.public;
    }
}
