use xaynet_core::mask::Aggregation;

use crate::state_machine::{
    phases::{Handler, Phase, PhaseName, PhaseState, PhaseStateError, Shared, Unmask},
    requests::{StateMachineRequest, Sum2Request},
    RequestError,
    StateMachine,
};

#[cfg(feature = "metrics")]
use crate::metrics;

use tokio::time::{timeout, Duration};

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
impl Phase for PhaseState<Sum2>
where
    Self: Handler,
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
            self.inner.sum2_count, self.shared.state.min_sum_count
        );
        Ok(())
    }

    /// Moves from the sum2 state to the next state.
    ///
    /// See the [module level documentation](../index.html) for more details.
    fn next(self) -> Option<StateMachine> {
        Some(PhaseState::<Unmask>::new(self.shared, self.inner.model_agg).into())
    }
}

impl PhaseState<Sum2>
where
    Self: Handler + Phase,
{
    /// Processes requests until there are enough.
    async fn process_until_enough(&mut self) -> Result<(), PhaseStateError> {
        while !self.has_enough_sum2s() {
            debug!(
                "{} sum2 messages handled (min {} required)",
                self.inner.sum2_count, self.shared.state.min_sum_count
            );
            self.process_single().await?;
        }
        Ok(())
    }
}

#[async_trait]
impl Handler for PhaseState<Sum2> {
    /// Handles a [`StateMachineRequest`],
    ///
    /// If the request is a [`StateMachineRequest::Sum`] or
    /// [`StateMachineRequest::Update`] request, the request sender
    /// will receive a [`RequestError::MessageRejected`].
    async fn handle_request(&mut self, req: StateMachineRequest) -> Result<(), RequestError> {
        match req {
            StateMachineRequest::Sum2(sum2_req) => {
                metrics!(
                    self.shared.io.metrics_tx,
                    metrics::message::sum2::increment(self.shared.state.round_id, Self::NAME)
                );
                self.handle_sum2(sum2_req).await
            }
            _ => Err(RequestError::MessageRejected),
        }
    }
}

impl PhaseState<Sum2> {
    /// Creates a new sum2 state.
    pub fn new(shared: Shared, model_agg: Aggregation) -> Self {
        Self {
            inner: Sum2 {
                model_agg,
                sum2_count: 0,
            },
            shared,
        }
    }

    /// Handles a sum2 request by adding a mask to the mask dictionary.
    ///
    /// # Errors
    /// Fails if the sum participant didn't register in the sum phase or it is a repetition.
    async fn handle_sum2(&mut self, req: Sum2Request) -> Result<(), RequestError> {
        let Sum2Request {
            participant_pk,
            model_mask,
        } = req;

        self.shared
            .io
            .redis
            .connection()
            .await
            .incr_mask_count(&participant_pk, &model_mask)
            .await?
            .into_inner()?;

        self.inner.sum2_count += 1;
        Ok(())
    }

    /// Checks whether enough sum participants submitted their masks to start the unmask phase.
    fn has_enough_sum2s(&self) -> bool {
        self.inner.sum2_count >= self.shared.state.min_sum_count
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::state_machine::{
        events::Event,
        tests::{builder::StateMachineBuilder, utils},
    };
    use serial_test::serial;
    use xaynet_core::{
        common::RoundSeed,
        crypto::{ByteObject, EncryptKeyPair},
        mask::{FromPrimitives, MaskConfig, Model},
        SumDict,
    };

    #[tokio::test]
    #[serial]
    pub async fn integration_sum2_to_unmask() {
        let n_updaters = 1;
        let n_summers = 1;
        let seed = RoundSeed::generate();
        let sum_ratio = 0.5;
        let update_ratio = 1.0;
        let coord_keys = EncryptKeyPair::generate();
        let model_size = 4;

        // Generate a sum dictionary with a single sum participant
        let mut summer = utils::generate_summer(&seed, sum_ratio, update_ratio);
        let ephm_pk = utils::ephm_pk(&summer.compose_sum_message(coord_keys.public));
        let mut sum_dict = SumDict::new();
        sum_dict.insert(summer.pk, ephm_pk);

        // Generate a new masked model, seed dictionary and aggregation
        let updater = utils::generate_updater(&seed, sum_ratio, update_ratio);
        let scalar = 1.0 / (n_updaters as f64 * update_ratio);
        let model = Model::from_primitives(vec![0; model_size].into_iter()).unwrap();
        let msg =
            updater.compose_update_message(coord_keys.public, &sum_dict, scalar, model.clone());
        let masked_model = utils::masked_model(&msg);
        let local_seed_dict = utils::local_seed_dict(&msg);
        let config: MaskConfig = utils::mask_settings().into();
        let mut aggregation = Aggregation::new(config.into(), model_size);
        aggregation.aggregate(masked_model.clone());

        // Create the state machine
        let sum2 = Sum2 {
            model_agg: aggregation,
            sum2_count: 0,
        };

        let (state_machine, request_tx, events, eio) = StateMachineBuilder::new()
            .await
            .with_seed(seed.clone())
            .with_phase(sum2)
            .with_sum_ratio(sum_ratio)
            .with_update_ratio(update_ratio)
            .with_min_sum(n_summers)
            .with_min_update(n_updaters)
            .with_min_sum_time(1)
            .with_max_sum_time(2)
            .with_mask_config(utils::mask_settings().into())
            .build();
        assert!(state_machine.is_sum2());

        // Write the sum participant into redis so that the mask lua script does not fail
        eio.redis
            .connection()
            .await
            .add_sum_participant(&summer.pk, &ephm_pk)
            .await
            .unwrap();

        // Create a sum2 request.
        let msg = summer
            .compose_sum2_message(
                coord_keys.public,
                &local_seed_dict,
                masked_model.vect.data.len(),
            )
            .unwrap();

        // Have the state machine process the request
        let req = async { request_tx.msg(&msg).await.unwrap() };
        let transition = async { state_machine.next().await.unwrap() };
        let ((), state_machine) = tokio::join!(req, transition);
        assert!(state_machine.is_unmask());

        // Extract state of the state machine
        let PhaseState {
            inner: unmask_state,
            ..
        } = state_machine.into_unmask_phase_state();

        // Check the initial state of the unmask phase.

        let mut best_masks = eio.redis.connection().await.get_best_masks().await.unwrap();
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
