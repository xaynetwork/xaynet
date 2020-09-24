use xaynet_core::{
    mask::{Aggregation, MaskObject},
    SumParticipantPublicKey,
};

use crate::state_machine::{
    coordinator::MaskDict,
    phases::{Handler, Phase, PhaseName, PhaseState, Shared, StateError, Unmask},
    requests::{StateMachineRequest, Sum2Request},
    StateMachine,
    StateMachineError,
};

#[cfg(feature = "metrics")]
use crate::metrics;

use tokio::time::{timeout, Duration};

/// Sum2 state
#[derive(Debug)]
pub struct Sum2 {
    /// The aggregator for masked models.
    model_agg: Aggregation,

    /// The aggregator for masked scalars.
    scalar_agg: Aggregation,

    /// The model mask dictionary built during the sum2 phase.
    model_mask_dict: MaskDict,

    /// The scalar mask dictionary built during the sum2 phase.
    scalar_mask_dict: MaskDict,
}

#[cfg(test)]
impl Sum2 {
    pub fn aggregation(&self) -> &Aggregation {
        &self.model_agg
    }

    pub fn mask_dict(&self) -> &MaskDict {
        &self.model_mask_dict
    }

    pub fn scalar_agg(&self) -> &Aggregation {
        &self.scalar_agg
    }

    pub fn scalar_mask_dict(&self) -> &MaskDict {
        &self.scalar_mask_dict
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
    async fn run(&mut self) -> Result<(), StateError> {
        let min_time = self.shared.state.min_sum_time;
        debug!("in sum2 phase for a minimum of {} seconds", min_time);
        self.process_during(Duration::from_secs(min_time)).await?;

        let time_left = self.shared.state.max_sum_time - min_time;
        timeout(Duration::from_secs(time_left), self.process_until_enough()).await??;

        info!(
            "{} sum2 messages handled (min {} required)",
            self.mask_count(),
            self.shared.state.min_sum_count
        );
        Ok(())
    }

    /// Moves from the sum2 state to the next state.
    ///
    /// See the [module level documentation](../index.html) for more details.
    fn next(self) -> Option<StateMachine> {
        Some(
            PhaseState::<Unmask>::new(
                self.shared,
                self.inner.model_agg,
                self.inner.scalar_agg,
                self.inner.model_mask_dict,
                self.inner.scalar_mask_dict,
            )
            .into(),
        )
    }
}

impl PhaseState<Sum2>
where
    Self: Handler + Phase,
{
    /// Processes requests until there are enough.
    async fn process_until_enough(&mut self) -> Result<(), StateError> {
        while !self.has_enough_sum2s() {
            debug!(
                "{} sum2 messages handled (min {} required)",
                self.mask_count(),
                self.shared.state.min_sum_count
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
    /// will receive a [`StateMachineError::MessageRejected`].
    async fn handle_request(&mut self, req: StateMachineRequest) -> Result<(), StateMachineError> {
        match req {
            StateMachineRequest::Sum2(sum2_req) => {
                metrics!(
                    self.shared.io.metrics_tx,
                    metrics::message::sum2::increment(self.shared.state.round_id, Self::NAME)
                );
                self.handle_sum2(sum2_req)
            }
            _ => Err(StateMachineError::MessageRejected),
        }
    }
}

impl PhaseState<Sum2> {
    /// Creates a new sum2 state.
    pub fn new(shared: Shared, model_agg: Aggregation, scalar_agg: Aggregation) -> Self {
        info!("state transition");
        Self {
            inner: Sum2 {
                model_agg,
                scalar_agg,
                model_mask_dict: MaskDict::new(),
                scalar_mask_dict: MaskDict::new(),
            },
            shared,
        }
    }

    /// Handles a sum2 request.
    /// If the handling of the sum2 message fails, an error is returned to the request sender.
    fn handle_sum2(&mut self, req: Sum2Request) -> Result<(), StateMachineError> {
        let Sum2Request {
            participant_pk,
            model_mask,
            scalar_mask,
        } = req;
        self.add_mask(&participant_pk, model_mask, scalar_mask)
    }

    /// Adds a mask to the mask dictionary.
    ///
    /// # Errors
    /// Fails if the sum participant didn't register in the sum phase or it is a repetition.
    fn add_mask(
        &mut self,
        _pk: &SumParticipantPublicKey,
        model_mask: MaskObject,
        scalar_mask: MaskObject,
    ) -> Result<(), StateMachineError> {
        // We remove the participant key here to make sure a participant
        // cannot submit a mask multiple times

        // FIXME: reactivate the check when the mask dict is moved into Redis
        // if self.inner.sum_dict.remove(pk).is_none() {
        //     return Err(StateMachineError::MessageRejected);
        // }

        if let Some(count) = self.inner.model_mask_dict.get_mut(&model_mask) {
            *count += 1;
        } else {
            self.inner.model_mask_dict.insert(model_mask, 1);
        }

        if let Some(count) = self.inner.scalar_mask_dict.get_mut(&scalar_mask) {
            *count += 1;
        } else {
            self.inner.scalar_mask_dict.insert(scalar_mask, 1);
        }

        Ok(())
    }

    fn mask_count(&self) -> usize {
        let sum1 = self.inner.model_mask_dict.values().sum();
        let sum2: usize = self.inner.scalar_mask_dict.values().sum();
        if sum1 != sum2 {
            warn!(
                "unexpected difference in mask sum count: {} vs {}",
                sum1, sum2
            );
        }
        sum1
    }

    /// Checks whether enough sum participants submitted their masks to start the idle phase.
    fn has_enough_sum2s(&self) -> bool {
        self.mask_count() >= self.shared.state.min_sum_count
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
        mask::{FromPrimitives, Model},
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
        let masked_scalar = utils::masked_scalar(&msg);
        let local_seed_dict = utils::local_seed_dict(&msg);
        let mut aggregation = Aggregation::new(utils::mask_settings().into(), model_size);
        aggregation.aggregate(masked_model.clone());
        let mut scalar_agg = Aggregation::new(utils::mask_settings().into(), 1);
        scalar_agg.aggregate(masked_scalar.clone());

        // Create the state machine
        let sum2 = Sum2 {
            model_agg: aggregation,
            scalar_agg,
            model_mask_dict: MaskDict::new(),
            scalar_mask_dict: MaskDict::new(),
        };

        let (state_machine, request_tx, events, _) = StateMachineBuilder::new()
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

        // Create a sum2 request.
        let msg = summer
            .compose_sum2_message(coord_keys.public, &local_seed_dict, masked_model.data.len())
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
                round_id: 0,
                event: PhaseName::Sum2,
            }
        );
    }
}
