use crate::service::Handle;
use crate::participant::{Participant, Task};
use sodiumoxide::crypto::box_;
use std::sync::Arc;

/// TODO
pub struct Client {
    handle: Handle,
    particip: Participant,
    coord_encr_pk: Option<box_::PublicKey>,
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
                coord_encr_pk: None,
            }
        }
    }

    /// TODO
    pub fn start(&mut self) {
        // self.pre_round()
    }

    async fn pre_round(&mut self) -> Option<()> {
        let round_params = self.handle
            .get_round_parameters()
            .await?;
        self.coord_encr_pk = Some(round_params.encr_pk);
        let round_seed: &[u8] = round_params.seed.as_slice();
        self.particip
            .compute_signatures(round_seed);
        let (sum_frac, upd_frac) = (round_params.sum, round_params.update);
        match self.particip.check_task(sum_frac, upd_frac) {
            Task::Sum    => self.summer().await,
            Task::Update => self.updater(),
            Task::None   => self.unselected(),
        }
    }

    fn unselected(&mut self) -> Option<()> {
        Some(())
    }

    async fn summer(&mut self) -> Option<()> {
        let sum1_msg: Vec<u8> = self.particip
            .compose_message_sum(&self.coord_encr_pk?);
        self.handle
            .send_message(sum1_msg)
            .await;
        let _pk = self.particip.get_encr_pk();
        let _seed_dict: Arc<Vec<u8>> = self.handle
            .get_seed_dict() // pass pk
            .await?;
        Some(())
    }

    fn updater(&mut self) -> Option<()> {
        Some(())
    }
}
