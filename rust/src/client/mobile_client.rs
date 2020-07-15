use crate::{
    client::{participant::Task, ClientError, Participant, Proxy},
    crypto::ByteObject,
    mask::model::{FromPrimitives, Model},
    CoordinatorPublicKey,
    PetError,
};
use derive_more::From;
use std::{
    cell::{RefCell, RefMut},
    rc::Rc,
    sync::{Arc, Mutex},
};

pub struct MobileClient {
    runtime: tokio::runtime::Runtime,
    local_model: Rc<RefCell<Option<Model>>>,
    global_model: Rc<RefCell<Option<Model>>>,
    participant: Option<StateMachine>,
}

impl MobileClient {
    pub fn new(proxy: Proxy) -> Self {
        let runtime = tokio::runtime::Builder::new()
            .basic_scheduler()
            .enable_all()
            .build()
            .unwrap();

        let local_model = Rc::new(RefCell::new(None));
        let global_model = Rc::new(RefCell::new(None));

        Self {
            runtime,
            local_model: local_model.clone(),
            global_model: global_model.clone(),
            participant: Some(StateMachine::new(proxy, local_model, global_model)),
        }
    }

    pub fn set_local_model(&self, local_model: Model) {
        *self.local_model.borrow_mut() = Some(local_model);
    }

    pub fn get_global_model(&self) -> Option<Model> {
        self.global_model.borrow().clone()
    }

    pub fn next(&mut self) {
        if let Some(participant) = self.participant.take() {
            let new_participant = self
                .runtime
                .block_on(async move { participant.next().await });
            self.participant = Some(new_participant);
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
    pub fn new(
        proxy: Proxy,
        local_model: Rc<RefCell<Option<Model>>>,
        global_model: Rc<RefCell<Option<Model>>>,
    ) -> Self {
        StateMachine::from(State::<Idle>::new(proxy, local_model, global_model))
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
    local_model: Rc<RefCell<Option<Model>>>,
    global_model: Rc<RefCell<Option<Model>>>,
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
    fn new(
        proxy: Proxy,
        local_model: Rc<RefCell<Option<Model>>>,
        global_model: Rc<RefCell<Option<Model>>>,
    ) -> Self {
        Self {
            task: Idle,
            proxy,
            participant: Participant::new().unwrap(),
            coordinator_pk: CoordinatorPublicKey::zeroed(),
            local_model,
            global_model,
        }
    }

    async fn next(mut self) -> StateMachine {
        match self.step().await {
            Ok(Task::Sum) => State::<Sum>::new(
                self.proxy,
                self.participant,
                self.coordinator_pk,
                self.local_model,
                self.global_model,
            )
            .into(),
            Ok(Task::Update) => State::<Update>::new(
                self.proxy,
                self.participant,
                self.coordinator_pk,
                self.local_model,
                self.global_model,
            )
            .into(),
            Ok(Task::None) | Err(_) => {
                State::<Idle>::new(self.proxy, self.local_model, self.global_model).into()
            }
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

    async fn fetch_global_model(&mut self) {
        if let Ok(model) = self.proxy.get_model().await {
            //update our global model where necessary

            let mut global_model = self.global_model.borrow_mut();

            match (model, global_model.as_ref()) {
                (Some(new_model), None) => {
                    debug!("new global model");
                    *global_model = Some(new_model);
                }
                (Some(new_model), Some(old_model)) if &new_model != old_model => {
                    debug!("updating global model");
                    *global_model = Some(new_model);
                }
                (None, _) => trace!("global model not ready yet"),
                _ => trace!("global model still fresh"),
            }
        }
    }
}

impl State<Sum> {
    fn new(
        proxy: Proxy,
        participant: Participant,
        coordinator_pk: CoordinatorPublicKey,
        local_model: Rc<RefCell<Option<Model>>>,
        global_model: Rc<RefCell<Option<Model>>>,
    ) -> Self {
        Self {
            task: Sum,
            proxy,
            participant,
            coordinator_pk,
            local_model,
            global_model,
        }
    }

    async fn next(mut self) -> StateMachine {
        info!("selected to sum");
        match self.step().await {
            Ok(_) => State::<Sum2>::new(
                self.proxy,
                self.participant,
                self.coordinator_pk,
                self.local_model,
                self.global_model,
            )
            .into(),
            Err(_) => State::<Idle>::new(self.proxy, self.local_model, self.global_model).into(),
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
    fn new(
        proxy: Proxy,
        participant: Participant,
        coordinator_pk: CoordinatorPublicKey,
        local_model: Rc<RefCell<Option<Model>>>,
        global_model: Rc<RefCell<Option<Model>>>,
    ) -> Self {
        Self {
            task: Update,
            proxy,
            participant,
            coordinator_pk,
            local_model,
            global_model,
        }
    }

    async fn next(self) -> StateMachine {
        info!("selected to update");
        State::<Idle>::new(self.proxy, self.local_model, self.global_model).into()
    }

    async fn step(&mut self) -> Result<(), ClientError> {
        self.check_round_freshness().await?;

        let local_model = self
            .local_model
            .borrow_mut()
            .take()
            .ok_or(ClientError::GeneralErr)?
            .clone();

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
            local_model,
        );
        self.proxy.post_message(upd_msg).await?;

        info!("update participant completed a round");
        Ok(())
    }
}

impl State<Sum2> {
    fn new(
        proxy: Proxy,
        participant: Participant,
        coordinator_pk: CoordinatorPublicKey,
        local_model: Rc<RefCell<Option<Model>>>,
        global_model: Rc<RefCell<Option<Model>>>,
    ) -> Self {
        Self {
            task: Sum2,
            proxy,
            participant,
            coordinator_pk,
            local_model,
            global_model,
        }
    }

    async fn next(self) -> StateMachine {
        info!("selected to sum2");
        State::<Idle>::new(self.proxy, self.local_model, self.global_model).into()
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
