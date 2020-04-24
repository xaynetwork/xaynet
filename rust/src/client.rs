use crate::service::Handle;
use crate::participant::{Participant, Task};
use crate::{CoordinatorPublicKey, SeedDict, SumDict};
use sodiumoxide::crypto::box_;
use std::sync::Arc;
use std::collections::HashMap;

use std::future::Future;
use std::pin::Pin;

/// TODO
pub struct Client {
    handle: Handle,
    particip: Participant,
    // coord_encr_pk: Option<box_::PublicKey>,
}

impl Client {
    /// Create a new `Client`
    pub fn new() -> Self {
        let (handle, _events) = Handle::new();
        match Participant::new() {
            Err(err) => panic!(err),
            Ok(particip) => Self {
                handle,
                particip,
                // coord_encr_pk: None,
            }
        }
    }

    /// TODO
    pub fn start(&mut self) {
        // self.pre_round()
    }

    fn pre_round(&mut self) -> Pin<Box<dyn Future<Output = Option<()>>>> {
        Box::pin(async move {
            let round_params = self.handle
                .get_round_parameters()
                .await?;
            let coord_pk = round_params.pk;
            let round_seed: &[u8] = round_params.seed.as_slice();
            self.particip
                .compute_signatures(round_seed);
            let (sum_frac, upd_frac) = (round_params.sum, round_params.update);
            match self.particip.check_task(sum_frac, upd_frac) {
                Task::Sum    =>
                    self.summer(coord_pk)
                    .await,
                Task::Update =>
                    self.updater(coord_pk)
                    .await,
                Task::None   =>
                    self.unselected()
                    .await,
            }
        })
    }

    async fn unselected(&mut self) -> Option<()> {
        // TODO await global model; save it
        // next round
        self.pre_round()
            .await
    }

    async fn summer(&mut self, coord_pk: CoordinatorPublicKey) -> Option<()> {
        let sum1_msg: Vec<u8> = self.particip
            .compose_sum_message(&coord_pk);
        self.handle
            .send_message(sum1_msg)
            .await;
        let _pk = self.particip.get_encr_pk();
        let _seed_dict: Arc<Vec<u8>> = self.handle
            .get_seed_dict() // later will need to pass pk
            .await?;
        // TODO deserialize the seed_dict
        let dummy_seed_dict: SeedDict = HashMap::new(); // FIXME
        // https://github.com/servo/bincode
        // bincode::deserialize(&seed_dict[..]).unwrap()
        let sum2_msg: Vec<u8> = self.particip
            .compose_sum2_message(&coord_pk, &dummy_seed_dict)
            .ok()?;
        self.handle
            .send_message(sum2_msg)
            .await;

        // job done, unselect
        self.unselected()
            .await
    }

    async fn updater(&mut self, coord_pk: CoordinatorPublicKey) -> Option<()> {
        // TODO train a model update...
        let _sum_dict: Arc<Vec<u8>> = self.handle
            .get_sum_dict()
            .await?;
        // TODO deserialise the sum dict
        let dummy_sum_dict: SumDict = HashMap::new(); // FIXME
        let upd_msg: Vec<u8> = self.particip
            .compose_update_message(&coord_pk, &dummy_sum_dict);
        self.handle
            .send_message(upd_msg)
            .await;

        // job done, unselect
        self.unselected()
            .await
    }
}
