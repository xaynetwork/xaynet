use crate::{
    mask::{masking::Aggregation, object::MaskObject},
    state_machine::{
        coordinator::{CoordinatorState, MaskDict},
        events::PhaseEvent,
        phases::{Handler, Phase, PhaseState, StateError, Unmask},
        requests::{Request, RequestReceiver, Sum2Request, Sum2Response},
        StateMachine,
    },
    PetError,
    SumDict,
    SumParticipantPublicKey,
};

use tokio::{sync::oneshot, time::Duration};

/// Sum2 state
#[derive(Debug)]
pub struct Sum2 {
    /// The sum dictionary built during the sum phase.
    sum_dict: SumDict,

    /// The aggregator for masks and masked models.
    aggregation: Aggregation,

    /// The mask dictionary built during the sum2 phase.
    mask_dict: MaskDict,
}

#[cfg(test)]
impl Sum2 {
    pub fn sum_dict(&self) -> &SumDict {
        &self.sum_dict
    }
    pub fn aggregation(&self) -> &Aggregation {
        &self.aggregation
    }
    pub fn mask_dict(&self) -> &MaskDict {
        &self.mask_dict
    }
}

#[async_trait]
impl<R> Phase<R> for PhaseState<R, Sum2>
where
    Self: Handler<R>,
    R: Send,
{
    /// Moves from the sum2 state to the next state.
    ///
    /// See the [module level documentation](../index.html) for more details.
    async fn next(mut self) -> Option<StateMachine<R>> {
        info!("starting sum2 phase");

        info!("broadcasting sum2 phase event");
        self.coordinator_state.events.broadcast_phase(
            self.coordinator_state.round_params.seed.clone(),
            PhaseEvent::Sum2,
        );
        let next_state = match self.run_phase().await {
            Ok(_) => PhaseState::<R, Unmask>::new(
                self.coordinator_state,
                self.request_rx,
                self.inner.aggregation,
                self.inner.mask_dict,
            )
            .into(),
            Err(err) => {
                PhaseState::<R, StateError>::new(self.coordinator_state, self.request_rx, err)
                    .into()
            }
        };
        Some(next_state)
    }
}

impl<R> Handler<Request> for PhaseState<R, Sum2> {
    /// Handles a [`Request::Sum`], [`Request::Update`] or [`Request::Sum2`] request.
    ///
    /// If the request is a [`Request::Sum`] or [`Request::Update`] request, the request sender
    /// will receive a [`PetError::InvalidMessage`].
    fn handle_request(&mut self, req: Request) {
        match req {
            Request::Sum2((sum2_req, response_tx)) => self.handle_sum2(sum2_req, response_tx),
            Request::Sum((_, response_tx)) => Self::handle_invalid_message(response_tx),
            Request::Update((_, response_tx)) => Self::handle_invalid_message(response_tx),
        }
    }
}

impl<R> PhaseState<R, Sum2>
where
    Self: Handler<R>,
{
    /// Runs the sum2 phase.
    async fn run_phase(&mut self) -> Result<(), StateError> {
        let min_time = self.coordinator_state.min_sum_time;
        debug!("in sum2 phase for a minimum of {} seconds", min_time);
        self.process_during(Duration::from_secs(min_time)).await?;

        while !self.has_enough_sum2s() {
            debug!(
                "{} sum2 messages handled (min {} required)",
                self.mask_count(),
                self.coordinator_state.min_sum_count
            );
            let req = self.next_request().await?;
            self.handle_request(req);
        }

        info!(
            "{} sum2 messages handled (min {} required)",
            self.mask_count(),
            self.coordinator_state.min_sum_count
        );
        Ok(())
    }
}

impl<R> PhaseState<R, Sum2> {
    /// Creates a new sum2 state.
    pub fn new(
        coordinator_state: CoordinatorState,
        request_rx: RequestReceiver<R>,
        sum_dict: SumDict,
        aggregation: Aggregation,
    ) -> Self {
        info!("state transition");
        Self {
            inner: Sum2 {
                sum_dict,
                aggregation,
                mask_dict: MaskDict::new(),
            },
            coordinator_state,
            request_rx,
        }
    }

    /// Handles a sum2 request.
    /// If the handling of the sum2 message fails, an error is returned to the request sender.
    fn handle_sum2(&mut self, req: Sum2Request, response_tx: oneshot::Sender<Sum2Response>) {
        let Sum2Request {
            participant_pk,
            mask,
        } = req;

        // See `Self::handle_invalid_message`
        let _ = response_tx.send(self.add_mask(&participant_pk, mask));
    }

    /// Adds a mask to the mask dictionary.
    ///
    /// # Errors
    /// Fails if the sum participant didn't register in the sum phase or it is a repetition.
    fn add_mask(&mut self, pk: &SumParticipantPublicKey, mask: MaskObject) -> Result<(), PetError> {
        // We move the participant key here to make sure a participant
        // cannot submit a mask multiple times
        if self.inner.sum_dict.remove(pk).is_none() {
            return Err(PetError::InvalidMessage);
        }

        if let Some(count) = self.inner.mask_dict.get_mut(&mask) {
            *count += 1;
        } else {
            self.inner.mask_dict.insert(mask, 1);
        }

        Ok(())
    }

    fn mask_count(&self) -> usize {
        self.inner.mask_dict.values().sum()
    }

    /// Checks whether enough sum participants submitted their masks to start the idle phase.
    fn has_enough_sum2s(&self) -> bool {
        self.mask_count() >= self.coordinator_state.min_sum_count
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        crypto::{ByteObject, EncryptKeyPair},
        mask::{FromPrimitives, Model},
        state_machine::{
            coordinator::RoundSeed,
            events::Event,
            tests::{
                builder::StateMachineBuilder,
                utils::{generate_summer, generate_updater, mask_settings},
            },
        },
        SumDict,
    };

    #[tokio::test]
    pub async fn sum2_to_unmask() {
        let n_updaters = 1;
        let n_summers = 1;
        let seed = RoundSeed::generate();
        let sum_ratio = 0.5;
        let update_ratio = 1.0;
        let coord_keys = EncryptKeyPair::generate();

        // Generate a sum dictionary with a single sum participant
        let mut summer = generate_summer(&seed, sum_ratio, update_ratio);
        let ephm_pk = summer.compose_sum_message(&coord_keys.public).ephm_pk();
        let mut sum_dict = SumDict::new();
        sum_dict.insert(summer.pk, *&ephm_pk);

        // Generate a new masked model, seed dictionary and aggregration
        let updater = generate_updater(&seed, sum_ratio, update_ratio);
        let scalar = 1.0 / (n_updaters as f64 * update_ratio);
        let model = Model::from_primitives(vec![0; 4].into_iter()).unwrap();
        let msg =
            updater.compose_update_message(coord_keys.public, &sum_dict, scalar, model.clone());
        let masked_model = msg.masked_model();
        let local_seed_dict = msg.local_seed_dict();
        let mut aggregation = Aggregation::new(mask_settings().into());
        aggregation.aggregate(masked_model.clone());

        // Create the state machine
        let sum2 = Sum2 {
            sum_dict,
            aggregation,
            mask_dict: MaskDict::new(),
        };

        let (state_machine, request_tx, events) = StateMachineBuilder::new()
            .with_seed(seed.clone())
            .with_phase(sum2)
            .with_sum_ratio(sum_ratio)
            .with_update_ratio(update_ratio)
            .with_min_sum(n_summers)
            .with_min_update(n_updaters)
            .with_expected_participants(n_updaters + n_summers)
            .with_mask_config(mask_settings().into())
            .build();
        assert!(state_machine.is_sum2());

        // Create a sum2 request.
        let msg = summer
            .compose_sum2_message(coord_keys.public, &local_seed_dict, masked_model.data.len())
            .unwrap();

        // Have the state machine process the request
        let req = async { request_tx.clone().sum2(&msg).await.unwrap() };
        let transition = async { state_machine.next().await.unwrap() };
        let ((), state_machine) = tokio::join!(req, transition);
        assert!(state_machine.is_unmask());

        // Extract state of the state machine
        let PhaseState {
            inner: unmask_state,
            ..
        } = state_machine.into_unmask_phase_state();

        // Check the initial state of the unmask phase.

        assert_eq!(unmask_state.mask_dict().len(), 1);
        let (mask, count) = unmask_state.mask_dict().iter().next().unwrap().clone();
        assert_eq!(*count, 1);

        let unmasked_model = unmask_state
            .aggregation()
            .unwrap()
            .clone()
            .unmask(mask.clone());
        assert_eq!(unmasked_model, model);

        assert_eq!(
            events.phase_listener().get_latest(),
            Event {
                round_id: seed.clone(),
                event: PhaseEvent::Sum2,
            }
        );
    }
}
