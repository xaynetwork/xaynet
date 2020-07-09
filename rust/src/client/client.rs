use crate::{
    client::{ClientError, Participant, Proxy, Task},
    crypto::ByteObject,
    mask::model::Model,
    state_machine::coordinator::RoundParameters,
    CoordinatorPublicKey,
    InitError,
    PetError,
};
use futures::{
    future::{Fuse, FutureExt},
    pin_mut,
    select,
};
use std::{default::Default, sync::Arc, time::Duration};
use tokio::{
    sync::{broadcast, mpsc, watch},
    time,
};
pub struct RoundParamFetcher {
    /// Coordinator public key
    coordinator_pk: CoordinatorPublicKey,
    round_param_tx: mpsc::UnboundedSender<RoundParameters>,

    global_model: Option<Model>,
    global_model_tx: watch::Sender<Option<Model>>,

    proxy: Arc<Proxy>,
}

impl RoundParamFetcher {
    pub fn new(
        proxy: Arc<Proxy>,
        global_model_tx: watch::Sender<Option<Model>>,
    ) -> (mpsc::UnboundedReceiver<RoundParameters>, Self) {
        let (round_param_tx, round_param_rx) = mpsc::unbounded_channel();
        (
            round_param_rx,
            Self {
                coordinator_pk: CoordinatorPublicKey::zeroed(),
                round_param_tx,
                global_model: None,
                global_model_tx,
                proxy,
            },
        )
    }

    pub async fn check_new_round_param(&mut self) {
        let mut interval = time::interval(Duration::from_secs(5));

        loop {
            if let Ok(round_params) = self.proxy.get_round_params().await {
                if round_params.pk != self.coordinator_pk {
                    debug!("new round parameters");
                    self.coordinator_pk = round_params.pk;
                    self.round_param_tx.send(round_params);
                    // we can also fetch the global model in a separate task
                    // but it is more efficient to do it after the coordinator released new
                    // round parameters(because we know that the coordinator started a new round)
                    self.fetch_global_model().await;
                }
            } else {
                error!("Error receiving round parameters")
            }

            interval.tick().await;
        }
    }

    async fn fetch_global_model(&mut self) {
        let model = self.proxy.get_model().await.unwrap();
        //update our global model where necessary
        match (model, &self.global_model) {
            (Some(new_model), None) => self.set_global_model(new_model),
            (Some(new_model), Some(old_model)) if &new_model != old_model => {
                self.set_global_model(new_model)
            }
            (None, _) => trace!("global model not ready yet"),
            _ => trace!("global model still fresh"),
        }
    }

    fn set_global_model(&mut self, model: Model) {
        debug!("updating global model");
        self.global_model = Some(model);
        self.global_model_tx.broadcast(self.global_model.clone());
    }
}
pub struct Client;

impl Client {
    pub async fn start(
        proxy: Arc<Proxy>,
        local_model_rx: watch::Receiver<Option<Model>>,
        global_model_tx: watch::Sender<Option<Model>>,
        mut shutdown_rx: broadcast::Receiver<()>,
    ) {
        let (mut fetcher_rx, mut fetcher) = RoundParamFetcher::new(proxy.clone(), global_model_tx);

        let join_handle_fetcher = tokio::spawn(async move {
            tokio::select! {
                _ = fetcher.check_new_round_param() => {}
                _ = shutdown_rx.recv() => {info!("shutdown fetcher")}
            }
        });

        let join_handle_task = tokio::spawn(async move {
            // idea from https://rust-lang.github.io/async-book/06_multiple_futures/03_select.html#concurrent-tasks-in-a-select-loop-with-fuse-and-futuresunordered
            let participant_task = Fuse::terminated();
            pin_mut!(participant_task);

            loop {
                select! {
                    new_round_param = fetcher_rx.recv().fuse() => {
                        // New round parameters have arrived-- start a new `participant_task`,
                        // dropping the old one.
                        match new_round_param {
                            Some(round_params) =>  participant_task.set(Self::run_participant_task(round_params, proxy.clone(), local_model_rx.clone()).fuse()),
                            // fetcher is dropped -> signal to shut down
                            None => {info!("shutdown task"); break;}
                        }
                    },
                    // Run the `participant_task`
                    _ = participant_task => {},
                    // something went wrong
                    complete => break,
                }
            }
        });

        tokio::join!(join_handle_fetcher, join_handle_task);
    }

    async fn run_participant_task(
        round_params: RoundParameters,
        proxy: Arc<Proxy>,
        local_model_rx: watch::Receiver<Option<Model>>,
    ) -> Result<(), ClientError> {
        let mut participant = Participant::new().map_err(ClientError::ParticipantInitErr)?;

        participant.compute_signatures(round_params.seed.as_slice());
        let (sum_frac, upd_frac) = (round_params.sum, round_params.update);

        // update the flag only after everything else is done such that the client can learn
        // via the API that a new round has started once all parameters are available
        let task = participant.check_task(sum_frac, upd_frac);

        let coordinator_pk = round_params.pk;
        return match task {
            Task::Sum => Self::summer(proxy, coordinator_pk, participant).await,
            Task::Update => Self::update(proxy, coordinator_pk, participant, local_model_rx).await,
            Task::None => Self::unselected().await,
        };
    }

    /// Work flow for sum participants.
    async fn summer(
        proxy: std::sync::Arc<Proxy>,
        coordinator_pk: CoordinatorPublicKey,
        mut participant: Participant,
    ) -> Result<(), ClientError> {
        info!("selected to sum");
        let mut interval = time::interval(Duration::from_secs(5));

        let sum1_msg = participant.compose_sum_message(&coordinator_pk);
        proxy.post_message(sum1_msg).await?;

        debug!("polling for model/mask length");
        let length = loop {
            if let Some(length) = proxy.get_mask_length().await? {
                if length > usize::MAX as u64 {
                    return Err(ClientError::ParticipantErr(PetError::InvalidModel));
                } else {
                    break length as usize;
                }
            }
            trace!("model/mask length not ready, retrying.");
            interval.tick().await;
        };

        debug!("sum message sent, polling for seed dict.");
        let seeds = loop {
            if let Some(seeds) = proxy.get_seeds(participant.pk).await? {
                break seeds;
            }
            trace!("seed dict not ready, retrying.");
            interval.tick().await;
        };

        debug!("seed dict received, sending sum2 message.");
        let sum2_msg = participant
            .compose_sum2_message(coordinator_pk, &seeds, length)
            .map_err(|e| {
                error!("failed to compose sum2 message with seeds: {:?}", &seeds);
                ClientError::ParticipantErr(e)
            })?;
        proxy.post_message(sum2_msg).await?;

        info!("sum participant completed a round");
        Ok(())
    }

    /// Work flow for update participants.
    async fn update(
        proxy: std::sync::Arc<Proxy>,
        coordinator_pk: CoordinatorPublicKey,
        participant: Participant,
        mut local_model_rx: watch::Receiver<Option<Model>>,
    ) -> Result<(), ClientError> {
        info!("selected to update");
        let mut interval = time::interval(Duration::from_secs(5));

        let local_model = loop {
            if let Some(local_model) = local_model_rx.recv().await.ok_or(ClientError::GeneralErr)? {
                break local_model;
            } else {
                warn!("local model not ready");
            }
        };

        debug!("polling for model scalar");
        let scalar = loop {
            if let Some(scalar) = proxy.get_scalar().await? {
                break scalar;
            }
            trace!("model scalar not ready, retrying.");
            interval.tick().await;
        };

        debug!("polling for sum dict");
        let sums = loop {
            if let Some(sums) = proxy.get_sums().await? {
                break sums;
            }
            trace!("sum dict not ready, retrying.");
            interval.tick().await;
        };

        debug!("sum dict received, sending update message.");
        let upd_msg =
            participant.compose_update_message(coordinator_pk, &sums, scalar, local_model);
        proxy.post_message(upd_msg).await?;

        info!("update participant completed a round");
        Ok(())
    }

    /// Work flow for unselected participants.
    async fn unselected() -> Result<(), ClientError> {
        debug!("not selected");
        Ok(())
    }
}

// async fn retry<R>(
//     period: u64,
//     fut: impl Future<Output = Result<Option<R>, ClientError>> + Copy,
// ) -> Result<R, ClientError> {
//     let mut interval = time::interval(Duration::from_secs(period));
//     loop {
//         if let Some(R) = fut.await? {
//             break Ok(R);
//         }
//         trace!("not ready, retrying.");
//         interval.tick().await;
//     }
// }
