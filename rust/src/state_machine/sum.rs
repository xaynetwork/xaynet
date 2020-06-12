use super::{
    error::Error,
    requests::SumRequest,
    update::Update,
    CoordinatorState,
    Request,
    State,
    StateError,
    StateMachine,
};
use crate::{crypto::generate_encrypt_key_pair, LocalSeedDict};
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct Sum;

impl State<Sum> {
    pub fn new(
        coordinator_state: CoordinatorState,
        request_rx: mpsc::UnboundedReceiver<Request>,
    ) -> StateMachine {
        info!("state transition");
        StateMachine::Sum(Self {
            _inner: Sum,
            coordinator_state,
            request_rx,
        })
    }

    pub async fn next(mut self) -> StateMachine {
        match self.run_phase().await {
            Ok(_) => State::<Update>::new(self.coordinator_state, self.request_rx),
            Err(err) => State::<Error>::new(self.coordinator_state, self.request_rx, err),
        }
    }

    async fn run_phase(&mut self) -> Result<(), StateError> {
        self.gen_round_keypair();

        while !self.has_enough_sums() {
            let req = self.next_request().await?;
            self.handle_request(req);
        }

        self.freeze_sum_dict();
        Ok(())
    }

    fn handle_request(&mut self, req: Request) {
        match req {
            Request::Sum(sum_req) => self.handle_sum(sum_req),
            Request::Update(update_req) => Self::handle_invalid_message(update_req.response_tx),
            Request::Sum2(sum2_req) => Self::handle_invalid_message(sum2_req.response_tx),
        }
    }

    fn handle_sum(&mut self, req: SumRequest) {
        let SumRequest {
            participant_pk,
            ephm_pk,
            response_tx,
        } = req;

        self.coordinator_state
            .sum_dict
            .insert(participant_pk, ephm_pk);
        // Is it ok to ignore the error here?
        let _ = response_tx.send(Ok(()));
    }

    /// Freeze the sum dictionary.
    fn freeze_sum_dict(&mut self) {
        self.coordinator_state.seed_dict = self
            .coordinator_state
            .sum_dict
            .keys()
            .map(|pk| (*pk, LocalSeedDict::new()))
            .collect();
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
        self.coordinator_state.sum_dict.len() >= self.coordinator_state.min_sum
    }
}
