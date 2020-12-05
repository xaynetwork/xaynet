use async_trait::async_trait;
use tokio::time::{timeout, Duration};
use tracing::{debug, info};

use crate::{
    metric,
    metrics::Measurement,
    state_machine::{
        phases::{Handler, Phase, PhaseName, PhaseState, PhaseStateError, Shared, Unmask},
        requests::{StateMachineRequest, Sum2Request},
        RequestError,
        StateMachine,
    },
    storage::{CoordinatorStorage, ModelStorage},
};
use xaynet_core::mask::Aggregation;

/// Sum2 state
#[derive(Debug)]
pub struct Sum2 {
    /// The aggregator for masked models.
    model_agg: Aggregation,

    /// The number of Sum2 messages successfully processed.
    sum2_count: u64,
}

#[cfg(test)]
impl Sum2 {
    pub fn aggregation(&self) -> &Aggregation {
        &self.model_agg
    }
}

#[async_trait]
impl<C, M> Phase<C, M> for PhaseState<Sum2, C, M>
where
    Self: Handler,
    C: CoordinatorStorage,
    M: ModelStorage,
{
    const NAME: PhaseName = PhaseName::Sum2;

    /// Run the sum2 phase
    ///
    /// See the [module level documentation](../index.html) for more details.
    async fn run(&mut self) -> Result<(), PhaseStateError> {
        let min_time = self.shared.state.min_sum_time;
        debug!("in sum2 phase for a minimum of {} seconds", min_time);
        self.process_during(Duration::from_secs(min_time)).await?;

        let time_left = self.shared.state.max_sum_time - min_time;
        timeout(Duration::from_secs(time_left), self.process_until_enough()).await??;

        info!(
            "{} sum2 messages handled (min {} required)",
            self.private.sum2_count, self.shared.state.min_sum_count
        );
        Ok(())
    }

    /// Moves from the sum2 state to the next state.
    ///
    /// See the [module level documentation](../index.html) for more details.
    fn next(self) -> Option<StateMachine<C, M>> {
        Some(PhaseState::<Unmask, _, _>::new(self.shared, self.private.model_agg).into())
    }
}

impl<C, M> PhaseState<Sum2, C, M>
where
    Self: Handler + Phase<C, M>,
    C: CoordinatorStorage,
    M: ModelStorage,
{
    /// Processes requests until there are enough.
    async fn process_until_enough(&mut self) -> Result<(), PhaseStateError> {
        while !self.has_enough_sum2s() {
            debug!(
                "{} sum2 messages handled (min {} required)",
                self.private.sum2_count, self.shared.state.min_sum_count
            );
            self.process_next().await?;
        }
        Ok(())
    }

    fn increment_message_metric(&self) {
        metric!(
            Measurement::MessageSum2,
            1,
            ("round_id", self.shared.state.round_id),
            ("phase", Self::NAME as u8)
        );
    }
}

#[async_trait]
impl<C, M> Handler for PhaseState<Sum2, C, M>
where
    C: CoordinatorStorage,
    M: ModelStorage,
{
    /// Handles a [`StateMachineRequest`],
    ///
    /// If the request is a [`StateMachineRequest::Sum`] or
    /// [`StateMachineRequest::Update`] request, the request sender
    /// will receive a [`RequestError::MessageRejected`].
    async fn handle_request(&mut self, req: StateMachineRequest) -> Result<(), RequestError> {
        match req {
            StateMachineRequest::Sum2(sum2_req) => self.handle_sum2(sum2_req).await.map(|res| {
                self.increment_message_metric();
                res
            }),
            _ => Err(RequestError::MessageRejected),
        }
    }
}

impl<C, M> PhaseState<Sum2, C, M>
where
    C: CoordinatorStorage,
    M: ModelStorage,
{
    /// Creates a new sum2 state.
    pub fn new(shared: Shared<C, M>, model_agg: Aggregation) -> Self {
        Self {
            private: Sum2 {
                model_agg,
                sum2_count: 0,
            },
            shared,
        }
    }

    /// Handles a sum2 request by adding a mask to the mask dictionary.
    ///
    /// # Errors
    ///
    /// Fails if the mask score cannot be incremented due to a PET or [`StorageError`].
    async fn handle_sum2(&mut self, req: Sum2Request) -> Result<(), RequestError> {
        let Sum2Request {
            participant_pk,
            model_mask,
        } = req;

        self.shared
            .store
            .incr_mask_score(&participant_pk, &model_mask)
            .await?
            .into_inner()?;

        self.private.sum2_count += 1;
        Ok(())
    }

    /// Checks whether enough sum participants submitted their masks to start the unmask phase.
    fn has_enough_sum2s(&self) -> bool {
        self.private.sum2_count >= self.shared.state.min_sum_count
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use serial_test::serial;
    use xaynet_core::{
        common::{RoundParameters, RoundSeed},
        crypto::{ByteObject, EncryptKeyPair},
        mask::{FromPrimitives, Model},
        SumDict,
    };

    use super::*;
    use crate::{
        state_machine::{
            events::Event,
            tests::{
                builder::StateMachineBuilder,
                utils::{self, Participant},
            },
        },
        storage::tests::init_store,
    };

    #[tokio::test]
    #[serial]
    pub async fn integration_sum2_to_unmask() {
        utils::enable_logging();
        let model_length = 4;
        let round_params = RoundParameters {
            pk: EncryptKeyPair::generate().public,
            sum: 0.5,
            update: 1.0,
            seed: RoundSeed::generate(),
            mask_config: utils::mask_config(),
            model_length,
        };

        let n_updaters = 1;
        let n_summers = 1;

        // Generate a sum dictionary with a single sum participant
        let summer = utils::generate_summer(round_params.clone());
        let mut sum_dict = SumDict::new();
        sum_dict.insert(summer.keys.public, summer.ephm_keys.public);

        // Generate a new masked model, seed dictionary and aggregation
        let updater = utils::generate_updater(round_params.clone());
        let scalar = 1.0 / (n_updaters as f64 * round_params.update);
        let model = Model::from_primitives(vec![0; model_length].into_iter()).unwrap();
        let (mask_seed, masked_model) = updater.compute_masked_model(&model, scalar);
        let local_seed_dict = Participant::build_seed_dict(&sum_dict, &mask_seed);

        // Build the update seed dict that we'll give to the sum
        // participant, so that they can compute a global mask.
        let mut update_seed_dict = HashMap::new();
        let encrypted_seed = local_seed_dict.get(&summer.keys.public).unwrap();
        update_seed_dict.insert(updater.keys.public, encrypted_seed.clone());

        // Create the state machine in the Sum2 phase
        let mut agg = Aggregation::new(summer.mask_settings, model_length);
        agg.aggregate(masked_model);

        let mut store = init_store().await;
        let (state_machine, request_tx, events) = StateMachineBuilder::new(store.clone())
            .with_seed(round_params.seed.clone())
            .with_phase(Sum2 {
                model_agg: agg,
                sum2_count: 0,
            })
            .with_sum_ratio(round_params.sum)
            .with_update_ratio(round_params.update)
            .with_min_sum(n_summers)
            .with_min_update(n_updaters)
            .with_min_sum_time(1)
            .with_max_sum_time(2)
            .with_mask_config(utils::mask_settings().into())
            .build();
        assert!(state_machine.is_sum2());

        // Write the sum participant into the store so that the method store.incr_mask_score does
        // not fail
        store
            .add_sum_participant(&summer.keys.public, &summer.ephm_keys.public)
            .await
            .unwrap();

        // aggregate the masks (there's only one), compose a sum2
        // message and have the state machine process it
        let seeds = summer.decrypt_seeds(&update_seed_dict);
        let aggregation = summer.aggregate_masks(model_length, &seeds);
        let msg = summer.compose_sum2_message(aggregation.clone().into());

        let req = async { request_tx.msg(&msg).await.unwrap() };
        let transition = async { state_machine.next().await.unwrap() };
        let ((), state_machine) = tokio::join!(req, transition);
        assert!(state_machine.is_unmask());

        // Extract state of the state machine
        let PhaseState {
            private: unmask_state,
            ..
        } = state_machine.into_unmask_phase_state();

        // Check the initial state of the unmask phase.
        let mut best_masks = store.best_masks().await.unwrap().unwrap();
        assert_eq!(best_masks.len(), 1);
        let (mask, count) = best_masks.pop().unwrap();
        assert_eq!(count, 1);

        let unmasked_model = unmask_state.aggregation().unwrap().clone().unmask(mask);
        assert_eq!(unmasked_model, model);

        assert_eq!(
            events.phase_listener().get_latest(),
            Event {
                round_id: 0,
                event: PhaseName::Sum2,
            }
        );
    }
}
