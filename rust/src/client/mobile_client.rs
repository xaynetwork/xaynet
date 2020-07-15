use crate::{
    client::{participant::Task, ClientError, Participant, Proxy},
    crypto::ByteObject,
    mask::model::{FromPrimitives, Model},
    CoordinatorPublicKey,
    PetError,
};
use derive_more::From;
use std::sync::{Arc, Mutex};

pub struct MobileClient {
    local_model: Option<Arc<Mutex<Model>>>,
    global_model: Option<Arc<Mutex<Model>>>,
    participant: StateMachine,
}

impl MobileClient {
    pub fn new(proxy: Proxy) -> Self {
        Self {
            local_model: None,
            global_model: None,
            participant: StateMachine::new(proxy),
        }
    }

    pub fn set_local_model(&self, local_model: Model) {
        if let Some(current_local_model) = &self.local_model {
            let mut new_local_model = current_local_model.lock().unwrap();
            *new_local_model = local_model;
        }
    }

    pub fn get_global_model(&self) -> Option<Model> {
        if let Some(global_model) = &self.global_model {
            Some(global_model.lock().unwrap().clone())
        } else {
            None
        }
    }
}

#[derive(From)]
pub enum StateMachine {
    Idle(State<Idle>),
    Sum(State<Sum>),
    Update(State<Update>),
    Sum2(State<Sum2>),
}

impl StateMachine {
    pub fn new(proxy: Proxy) -> Self {
        StateMachine::from(State::<Idle>::new(proxy))
    }

    pub async fn next(self) -> Self {
        match self {
            StateMachine::Idle(state) => state.next().await,
            StateMachine::Sum(state) => state.next().await,
            StateMachine::Update(state) => state.next().await,
            StateMachine::Sum2(state) => state.next().await,
        }
    }
}

pub struct Idle; // unselected participant
pub struct Sum;
pub struct Update;
pub struct Sum2;

pub struct State<Task> {
    task: Task,
    proxy: Proxy,
    participant: Participant,
    coordinator_pk: CoordinatorPublicKey,
}

impl<Task> State<Task> {
    async fn check_round_freshness(&self) -> Result<(), ClientError> {
        let round_params = self.proxy.get_round_params().await?;
        if round_params.pk != self.coordinator_pk {
            debug!("new round parameters");
            Err(ClientError::GeneralErr) //old round
        } else {
            Ok(())
        }
    }
}

impl State<Idle> {
    fn new(proxy: Proxy) -> Self {
        Self {
            task: Idle,
            participant: Participant::new().unwrap(),
            coordinator_pk: CoordinatorPublicKey::zeroed(),
            proxy,
        }
    }

    async fn next(mut self) -> StateMachine {
        match self.step().await {
            Ok(Task::Sum) => {
                State::<Sum>::new(self.proxy, self.participant, self.coordinator_pk).into()
            }
            Ok(Task::Update) => {
                State::<Update>::new(self.proxy, self.participant, self.coordinator_pk).into()
            }
            Ok(Task::None) => State::<Idle>::new(self.proxy).into(),
            Err(_) => State::<Idle>::new(self.proxy).into(),
        }
    }

    async fn step(&mut self) -> Result<Task, ClientError> {
        let round_params = self.proxy.get_round_params().await?;
        self.coordinator_pk = round_params.pk;
        self.participant = Participant::new().unwrap();

        let model = self.proxy.get_model().await?;

        self.participant
            .compute_signatures(round_params.seed.as_slice());
        Ok(self
            .participant
            .check_task(round_params.sum, round_params.update))
    }
}

impl State<Sum> {
    fn new(proxy: Proxy, participant: Participant, coordinator_pk: CoordinatorPublicKey) -> Self {
        Self {
            task: Sum,
            participant,
            coordinator_pk,
            proxy,
        }
    }

    async fn next(mut self) -> StateMachine {
        info!("selected to sum");
        match self.step().await {
            Ok(_) => State::<Sum2>::new(self.proxy, self.participant, self.coordinator_pk).into(),
            Err(_) => State::<Idle>::new(self.proxy).into(),
        }
    }

    async fn step(&mut self) -> Result<(), ClientError> {
        self.check_round_freshness().await?;
        let sum1_msg = self.participant.compose_sum_message(&self.coordinator_pk);
        self.proxy.post_message(sum1_msg).await?;
        debug!("sum message sent");
        Ok(())
    }
}

impl State<Update> {
    fn new(proxy: Proxy, participant: Participant, coordinator_pk: CoordinatorPublicKey) -> Self {
        Self {
            task: Update,
            participant,
            coordinator_pk,
            proxy,
        }
    }

    async fn next(self) -> StateMachine {
        info!("selected to update");
        State::<Idle>::new(self.proxy).into()
    }

    async fn step(&mut self) -> Result<(), ClientError> {
        self.check_round_freshness().await?;

        // let local_model = loop {
        //     if let Some(local_model) = local_model_rx.recv().await.ok_or(ClientError::GeneralErr)? {
        //         break local_model;
        //     } else {
        //         warn!("local model not ready");
        //     }
        // };

        debug!("polling for model scalar");
        let scalar = self
            .proxy
            .get_scalar()
            .await?
            .ok_or(ClientError::GeneralErr)?;

        debug!("polling for sum dict");
        let sums = self
            .proxy
            .get_sums()
            .await?
            .ok_or(ClientError::GeneralErr)?;

        debug!("sum dict received, sending update message.");
        let upd_msg = self.participant.compose_update_message(
            self.coordinator_pk,
            &sums,
            scalar,
            Model::from_primitives(vec![0; 4].into_iter()).unwrap(),
        );
        self.proxy.post_message(upd_msg).await?;

        info!("update participant completed a round");
        Ok(())
    }
}

impl State<Sum2> {
    fn new(proxy: Proxy, participant: Participant, coordinator_pk: CoordinatorPublicKey) -> Self {
        Self {
            task: Sum2,
            participant,
            coordinator_pk,
            proxy,
        }
    }

    async fn next(self) -> StateMachine {
        info!("selected to sum2");
        State::<Idle>::new(self.proxy).into()
    }

    async fn step(&mut self) -> Result<(), ClientError> {
        self.check_round_freshness().await?;

        debug!("polling for model/mask length");
        let length = self
            .proxy
            .get_mask_length()
            .await?
            .ok_or(ClientError::GeneralErr)?;
        if length > usize::MAX as u64 {
            return Err(ClientError::ParticipantErr(PetError::InvalidModel));
        };

        let seeds = self
            .proxy
            .get_seeds(self.participant.pk)
            .await?
            .ok_or(ClientError::GeneralErr)?;

        debug!("seed dict received, sending sum2 message.");
        let sum2_msg = self
            .participant
            .compose_sum2_message(self.coordinator_pk, &seeds, length as usize)
            .map_err(|e| {
                error!("failed to compose sum2 message with seeds: {:?}", &seeds);
                ClientError::ParticipantErr(e)
            })?;
        self.proxy.post_message(sum2_msg).await?;
        info!("sum participant completed a round");
        Ok(())
    }
}
