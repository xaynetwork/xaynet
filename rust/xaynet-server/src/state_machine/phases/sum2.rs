use async_trait::async_trait;

use crate::{
    impl_handler_for_phasestate,
    state_machine::{
        phases::{Handler, Phase, PhaseName, PhaseState, PhaseStateError, Shared, Unmask},
        requests::{StateMachineRequest, Sum2Request},
        RequestError,
        StateMachine,
    },
    storage::Storage,
};
use xaynet_core::{
    mask::{Aggregation, MaskObject},
    SumParticipantPublicKey,
};

/// The sum2 state.
#[derive(Debug)]
pub struct Sum2 {
    /// The aggregator for masked models.
    model_agg: Aggregation,
    /// The number of sum2 messages successfully processed.
    accepted: u64,
    /// The number of sum2 messages failed to processed.
    rejected: u64,
    /// The number of sum2 messages discarded without being processed.
    discarded: u64,
}

#[async_trait]
impl<S> Phase<S> for PhaseState<Sum2, S>
where
    Self: Handler,
    S: Storage,
{
    const NAME: PhaseName = PhaseName::Sum2;

    async fn run(&mut self) -> Result<(), PhaseStateError> {
        self.process(self.shared.state.sum2).await
    }

    fn next(self) -> Option<StateMachine<S>> {
        Some(PhaseState::<Unmask, _>::new(self.shared, self.private.model_agg).into())
    }
}

#[async_trait]
impl<S> Handler for PhaseState<Sum2, S>
where
    S: Storage,
{
    async fn handle_request(&mut self, req: StateMachineRequest) -> Result<(), RequestError> {
        if let StateMachineRequest::Sum2(Sum2Request {
            participant_pk,
            model_mask,
        }) = req
        {
            self.update_mask_dict(participant_pk, model_mask).await
        } else {
            Err(RequestError::MessageRejected)
        }
    }

    impl_handler_for_phasestate! { Sum2 }
}

impl<S> PhaseState<Sum2, S>
where
    S: Storage,
{
    /// Creates a new sum2 state.
    pub fn new(shared: Shared<S>, model_agg: Aggregation) -> Self {
        Self {
            private: Sum2 {
                model_agg,
                accepted: 0,
                rejected: 0,
                discarded: 0,
            },
            shared,
        }
    }

    /// Updates the mask dict with a sum2 participant request.
    async fn update_mask_dict(
        &mut self,
        participant_pk: SumParticipantPublicKey,
        model_mask: MaskObject,
    ) -> Result<(), RequestError> {
        self.shared
            .store
            .incr_mask_score(&participant_pk, &model_mask)
            .await?
            .into_inner()
            .map_err(RequestError::from)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serial_test::serial;

    use super::*;
    use crate::{
        state_machine::{
            events::Event,
            tests::{
                builder::StateMachineBuilder,
                utils::{self, Participant},
            },
        },
        storage::{tests::init_store, CoordinatorStorage},
    };
    use xaynet_core::{
        common::{RoundParameters, RoundSeed},
        crypto::{ByteObject, EncryptKeyPair},
        mask::{FromPrimitives, Model},
        SumDict,
    };

    impl Sum2 {
        pub fn aggregation(&self) -> &Aggregation {
            &self.model_agg
        }
    }

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
                accepted: 0,
                rejected: 0,
                discarded: 0,
            })
            .with_sum_probability(round_params.sum)
            .with_update_probability(round_params.update)
            .with_sum_count_min(n_summers)
            .with_sum_count_max(n_summers + 10)
            .with_update_count_min(n_updaters)
            .with_update_count_max(n_updaters + 10)
            .with_sum2_count_min(n_summers)
            .with_sum2_count_max(n_summers + 10)
            .with_sum2_time_min(1)
            .with_sum2_time_max(2)
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
