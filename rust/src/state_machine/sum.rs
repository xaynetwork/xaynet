use super::{
    requests::SumRequest,
    update::Update,
    CoordinatorState,
    PhaseState,
    Request,
    SeedDict,
    StateError,
    StateMachine,
};
use crate::{crypto::generate_encrypt_key_pair, LocalSeedDict, SumDict};
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct Sum {
    /// Dictionary built during the sum phase.
    sum_dict: SumDict,
}

impl PhaseState<Sum> {
    pub fn new(
        coordinator_state: CoordinatorState,
        request_rx: mpsc::UnboundedReceiver<Request>,
    ) -> Self {
        info!("state transition");
        Self {
            inner: Sum {
                sum_dict: SumDict::new(),
            },
            coordinator_state,
            request_rx,
        }
    }

    pub async fn next(mut self) -> Option<StateMachine> {
        let next_state = match self.run_phase().await {
            Ok(seed_dict) => PhaseState::<Update>::new(
                self.coordinator_state,
                self.request_rx,
                self.inner.sum_dict,
                seed_dict,
            )
            .into(),
            Err(err) => {
                PhaseState::<StateError>::new(self.coordinator_state, self.request_rx, err).into()
            }
        };
        Some(next_state)
    }

    async fn run_phase(&mut self) -> Result<SeedDict, StateError> {
        self.gen_round_keypair();

        while !self.has_enough_sums() {
            let req = self.next_request().await?;
            self.handle_request(req);
        }

        // the scalar must be published later for the participants
        let _scalar = 1_f64
            / (self.coordinator_state.expected_participants as f64 * self.coordinator_state.update);
        Ok(self.freeze_sum_dict())
    }

    /// Handle a sum, update or sum2 request.
    /// If the request is a update or sum2 request, the receiver of the response channel will
    /// receive a [`PetError::InvalidMessage`].
    fn handle_request(&mut self, req: Request) {
        match req {
            Request::Sum(sum_req) => self.handle_sum(sum_req),
            Request::Update(update_req) => Self::handle_invalid_message(update_req.response_tx),
            Request::Sum2(sum2_req) => Self::handle_invalid_message(sum2_req.response_tx),
        }
    }

    /// Handle a sum request.
    fn handle_sum(&mut self, req: SumRequest) {
        let SumRequest {
            participant_pk,
            ephm_pk,
            response_tx,
        } = req;

        self.inner.sum_dict.insert(participant_pk, ephm_pk);

        // See `Self::handle_invalid_message`
        let _ = response_tx.send(Ok(()));
    }

    /// Freeze the sum dictionary.
    fn freeze_sum_dict(&mut self) -> SeedDict {
        self.inner
            .sum_dict
            .keys()
            .map(|pk| (*pk, LocalSeedDict::new()))
            .collect()
    }

    /// Generate fresh round credentials.
    fn gen_round_keypair(&mut self) {
        let (pk, sk) = generate_encrypt_key_pair();
        self.coordinator_state.pk = pk;
        self.coordinator_state.sk = sk;
    }

    /// Check whether enough sum participants submitted their ephemeral keys to start the update
    /// phase.
    fn has_enough_sums(&self) -> bool {
        self.inner.sum_dict.len() >= self.coordinator_state.min_sum
    }
}
