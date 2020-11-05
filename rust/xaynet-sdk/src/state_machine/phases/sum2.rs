use xaynet_core::{
    crypto::{EncryptKeyPair, Signature},
    mask::{Aggregation, MaskObject, MaskSeed},
    message::Sum2 as Sum2Message,
    UpdateSeedDict,
};

use crate::{
    state_machine::{Phase, Progress, Step, TransitionOutcome, IO},
    MessageEncoder,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct Sum2 {
    ephm_keys: EncryptKeyPair,
    sum_signature: Signature,
    seed_dict: Option<UpdateSeedDict>,
    seeds: Option<Vec<MaskSeed>>,
    mask: Option<MaskObject>,
    mask_length: Option<u64>,
    message: Option<MessageEncoder>,
}

impl Sum2 {
    pub fn new(ephm_keys: EncryptKeyPair, sum_signature: Signature) -> Self {
        Self {
            ephm_keys,
            sum_signature,
            seed_dict: None,
            seeds: None,
            mask: None,
            mask_length: None,
            message: None,
        }
    }

    fn has_fetched_seed_dict(&self) -> bool {
        self.seed_dict.is_some() || self.has_fetched_mask_length()
    }

    fn has_fetched_mask_length(&self) -> bool {
        self.mask_length.is_some() || self.has_decrypted_seeds()
    }

    fn has_decrypted_seeds(&self) -> bool {
        self.seeds.is_some() || self.has_aggregated_masks()
    }

    fn has_aggregated_masks(&self) -> bool {
        self.mask.is_some() || self.has_composed_message()
    }

    fn has_composed_message(&self) -> bool {
        self.message.is_some()
    }
}

impl Phase<Sum2> {
    async fn fetch_seed_dict(mut self) -> Progress<Sum2> {
        if self.state.private.has_fetched_seed_dict() {
            return Progress::Continue(self);
        }
        debug!("polling for update seeds");
        match self.io.get_seeds(self.state.shared.keys.public).await {
            Err(e) => {
                warn!("failed to fetch seeds: {}", e);
                Progress::Stuck(self)
            }
            Ok(None) => {
                debug!("seeds not available yet");
                Progress::Stuck(self)
            }
            Ok(Some(seeds)) => {
                self.state.private.seed_dict = Some(seeds);
                Progress::Updated(self.into())
            }
        }
    }

    async fn fetch_mask_length(mut self) -> Progress<Sum2> {
        if self.state.private.has_fetched_mask_length() {
            return Progress::Continue(self);
        }

        debug!("polling for mask length");
        match self.io.get_mask_length().await {
            Err(e) => {
                warn!("failed to fetch mask length: {}", e);
                Progress::Stuck(self)
            }
            Ok(None) => {
                debug!("mask length not available yet");
                Progress::Stuck(self)
            }
            Ok(Some(length)) => {
                self.state.private.mask_length = Some(length);
                Progress::Updated(self.into())
            }
        }
    }

    fn decrypt_seeds(mut self) -> Progress<Sum2> {
        if self.state.private.has_decrypted_seeds() {
            return Progress::Continue(self);
        }

        let keys = &self.state.private.ephm_keys;
        // UNWRAP_SAFE: the seed dict is set in
        // `self.fetch_seed_dict()` which is called before this method
        let seeds: Result<Vec<MaskSeed>, ()> = self
            .state
            .private
            .seed_dict
            .take()
            .unwrap()
            .into_iter()
            .map(|(_, seed)| seed.decrypt(&keys.public, &keys.secret).map_err(|_| ()))
            .collect();

        match seeds {
            Ok(seeds) => {
                self.state.private.seeds = Some(seeds);
                Progress::Updated(self.into())
            }
            Err(_) => {
                warn!("failed to decrypt mask seeds, going back to waiting phase");
                self.io.notify_idle();
                Progress::Updated(self.into_awaiting().into())
            }
        }
    }

    fn aggregate_masks(mut self) -> Progress<Sum2> {
        if self.state.private.has_aggregated_masks() {
            return Progress::Continue(self);
        }

        info!("aggregating masks");
        let config = self.state.shared.mask_config;
        // UNWRAP_SAFE: the mask length is set in
        // `self.fetch_mask_length()` which is called before this method
        let mask_len = self.state.private.mask_length.unwrap();
        let mut mask_agg = Aggregation::new(config, mask_len as usize);
        // UNWRAP_SAFE: the seeds are set in `self.decrypt_seeds()`
        // which is called before this method
        for seed in self.state.private.seeds.take().unwrap().into_iter() {
            let mask = seed.derive_mask(mask_len as usize, config);
            if let Err(e) = mask_agg.validate_aggregation(&mask) {
                error!("sum2 phase failed: cannot aggregate masks: {}", e);
                error!("going to awaiting phase");
                return Progress::Updated(self.into_awaiting().into());
            } else {
                mask_agg.aggregate(mask);
            }
        }
        self.state.private.mask = Some(mask_agg.into());
        Progress::Updated(self.into())
    }

    fn compose_sum2_message(mut self) -> Progress<Sum2> {
        if self.state.private.has_composed_message() {
            return Progress::Continue(self);
        }

        let sum2 = Sum2Message {
            sum_signature: self.state.private.sum_signature,
            // UNWRAP_SAFE: the mask set in `self.aggregate_masks()`
            // which is called before this method
            model_mask: self.state.private.mask.take().unwrap(),
        };
        self.state.private.message = Some(self.message_encoder(sum2.into()));
        Progress::Updated(self.into())
    }
}

#[async_trait]
impl Step for Phase<Sum2> {
    async fn step(mut self) -> TransitionOutcome {
        info!("sum2 task");
        self = try_progress!(self.fetch_seed_dict().await);
        self = try_progress!(self.fetch_mask_length().await);
        self = try_progress!(self.decrypt_seeds());
        self = try_progress!(self.aggregate_masks());
        self = try_progress!(self.compose_sum2_message());

        // FIXME: currently if sending fails, we lose the message,
        // thus wasting all the work we've done in this phase
        let message = self.state.private.message.take().unwrap();
        match self.send_message(message).await {
            Ok(_) => {
                info!("sent sum2 message");
            }
            Err(e) => {
                warn!("failed to send sum2 message: {}", e);
                warn!("sum2 phase failed");
            }
        }

        info!("going back to awaiting phase");
        TransitionOutcome::Complete(self.into_awaiting().into())
    }
}
