use crate::{
    crypto::{encrypt::EncryptKeyPair, sign::SigningKeySeed, ByteObject},
    state_machine::{
        coordinator::{CoordinatorState, RoundSeed},
        events::{DictionaryUpdate, MaskLengthUpdate, PhaseEvent, ScalarUpdate},
        phases::{Handler, Phase, PhaseState, Sum},
        requests::{Request, RequestReceiver},
        StateMachine,
    },
};

use sodiumoxide::crypto::hash::sha256;

/// Idle state
#[derive(Debug)]
pub struct Idle;

impl<R> Handler<Request> for PhaseState<R, Idle> {
    /// Reject all the request with a [`PetError::InvalidMessage`]
    fn handle_request(&mut self, req: Request) {
        match req {
            Request::Sum((_, response_tx)) => Self::handle_invalid_message(response_tx),
            Request::Update((_, response_tx)) => Self::handle_invalid_message(response_tx),
            Request::Sum2((_, response_tx)) => Self::handle_invalid_message(response_tx),
        }
    }
}

#[async_trait]
impl<R> Phase<R> for PhaseState<R, Idle>
where
    R: Send,
{
    /// Moves from the idle state to the next state.
    ///
    /// See the [module level documentation](../index.html) for more details.
    async fn next(mut self) -> Option<StateMachine<R>> {
        info!("starting idle phase");

        info!("updating the keys");
        self.gen_round_keypair();

        info!("updating round thresholds");
        self.update_round_thresholds();

        info!("updating round seeds");
        self.update_round_seed();

        let events = &mut self.coordinator_state.events;

        info!("broadcasting new keys");
        events.broadcast_keys(
            self.coordinator_state.round_params.seed.clone(),
            self.coordinator_state.keys.clone(),
        );

        info!("broadcasting idle phase event");
        events.broadcast_phase(
            self.coordinator_state.round_params.seed.clone(),
            PhaseEvent::Idle,
        );

        info!("broadcasting invalidation of sum dictionary from previous round");
        events.broadcast_sum_dict(
            self.coordinator_state.round_params.seed.clone(),
            DictionaryUpdate::Invalidate,
        );

        info!("broadcasting invalidation of seed dictionary from previous round");
        events.broadcast_seed_dict(
            self.coordinator_state.round_params.seed.clone(),
            DictionaryUpdate::Invalidate,
        );

        info!("broadcasting invalidation of scalar from previous round");
        events.broadcast_scalar(
            self.coordinator_state.round_params.seed.clone(),
            ScalarUpdate::Invalidate,
        );

        info!("broadcasting invalidation of mask length from previous round");
        events.broadcast_mask_length(
            self.coordinator_state.round_params.seed.clone(),
            MaskLengthUpdate::Invalidate,
        );

        info!("broadcasting new round parameters");
        events.broadcast_params(self.coordinator_state.round_params.clone());

        // TODO: add a delay to prolongate the idle phase

        info!("going to sum phase");
        Some(PhaseState::<R, Sum>::new(self.coordinator_state, self.request_rx).into())
    }
}

impl<R> PhaseState<R, Idle> {
    /// Creates a new idle state.
    pub fn new(coordinator_state: CoordinatorState, request_rx: RequestReceiver<R>) -> Self {
        Self {
            inner: Idle,
            coordinator_state,
            request_rx,
        }
    }

    fn update_round_thresholds(&mut self) {}

    /// Updates the seed round parameter.
    fn update_round_seed(&mut self) {
        // Safe unwrap: `sk` and `seed` have same number of bytes
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

    /// Generates fresh round credentials.
    fn gen_round_keypair(&mut self) {
        self.coordinator_state.keys = EncryptKeyPair::generate();
        self.coordinator_state.round_params.pk = self.coordinator_state.keys.public;
    }
}
