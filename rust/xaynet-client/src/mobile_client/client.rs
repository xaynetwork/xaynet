use crate::{
    api::ApiClient,
    mobile_client::participant::{
        Awaiting,
        Participant,
        ParticipantSettings,
        Role,
        Sum,
        Sum2,
        Update,
    },
    ClientError,
};
use derive_more::From;
use xaynet_core::{common::RoundParameters, crypto::ByteObject, mask::Model, InitError, PetError};

#[async_trait]
pub trait LocalModel {
    async fn get_local_model(&mut self) -> Option<Model>;
}

#[derive(Serialize, Deserialize)]
pub struct ClientState<Type> {
    participant: Participant<Type>,
    round_params: RoundParameters,
}

impl<Type> ClientState<Type> {
    async fn check_round_freshness<T: ApiClient>(
        &self,
        api: &mut T,
    ) -> Result<(), ClientError<T::Error>> {
        debug!("fetching round parameters");
        let round_params = api.get_round_params().await?;
        if round_params.seed != self.round_params.seed {
            info!("new round parameters");
            Err(ClientError::RoundOutdated)
        } else {
            Ok(())
        }
    }

    fn reset(self) -> ClientState<Awaiting> {
        warn!("reset client");
        ClientState::<Awaiting>::new(self.participant.reset(), self.round_params)
    }
}

impl ClientState<Awaiting> {
    fn new(participant: Participant<Awaiting>, round_params: RoundParameters) -> Self {
        Self {
            participant,
            round_params,
        }
    }

    async fn next<T: ApiClient>(mut self, api: &mut T) -> ClientStateMachine {
        info!("awaiting task");
        let new_round_param = match api.get_round_params().await {
            Ok(new_round_param) => new_round_param,
            Err(err) => {
                error!("{:?}", err);
                return self.reset().into();
            }
        };

        if new_round_param == self.round_params {
            debug!("still same round");
            return self.into();
        } else {
            self.round_params = new_round_param;
        }

        let Self {
            participant,
            round_params,
        } = self;

        match participant.determine_role(
            round_params.seed.as_slice(),
            round_params.sum,
            round_params.update,
        ) {
            Role::Unselected(participant) => {
                info!("unselected");
                ClientState::<Awaiting>::new(participant.reset(), round_params).into()
            }
            Role::Summer(participant) => ClientState::<Sum>::new(participant, round_params).into(),
            Role::Updater(participant) => {
                ClientState::<Update>::new(participant, round_params).into()
            }
        }
    }
}

impl ClientState<Sum> {
    fn new(participant: Participant<Sum>, round_params: RoundParameters) -> Self {
        Self {
            participant,
            round_params,
        }
    }

    async fn next<T: ApiClient>(mut self, api: &mut T) -> ClientStateMachine {
        info!("selected to sum");

        match self.run(api).await {
            Ok(_) => self.into_sum2().into(),
            Err(ClientError::RoundOutdated) => self.reset().into(),
            Err(err) => {
                error!("{:?}", err);
                self.into()
            }
        }
    }

    async fn run<T: ApiClient>(&mut self, api: &mut T) -> Result<(), ClientError<T::Error>> {
        self.check_round_freshness(api).await?;

        let sum_msg = self.participant.compose_sum_message(self.round_params.pk);
        let sealed_msg = self
            .participant
            .seal_message(&self.round_params.pk, &sum_msg);

        debug!("sending sum message");
        api.send_message(sealed_msg).await?;
        debug!("sum message sent");
        Ok(())
    }

    fn into_sum2(self) -> ClientState<Sum2> {
        ClientState::<Sum2>::new(self.participant.into(), self.round_params)
    }
}

impl ClientState<Update> {
    fn new(participant: Participant<Update>, round_params: RoundParameters) -> Self {
        Self {
            participant,
            round_params,
        }
    }

    async fn next<L: LocalModel, T: ApiClient>(
        mut self,
        api: &mut T,
        local_model: &mut L,
    ) -> ClientStateMachine {
        info!("selected to update");

        match self.run(api, local_model).await {
            Ok(_) | Err(ClientError::RoundOutdated) => self.reset().into(),
            Err(err) => {
                error!("{:?}", err);
                self.into()
            }
        }
    }

    async fn run<L: LocalModel, T: ApiClient>(
        &mut self,
        api: &mut T,
        local_model: &mut L,
    ) -> Result<(), ClientError<T::Error>> {
        self.check_round_freshness(api).await?;

        debug!("polling for local model");
        let local_model = local_model
            .get_local_model()
            .await
            .ok_or(ClientError::TooEarly("local model"))?;

        debug!("polling for sum dict");
        let sums = api
            .get_sums()
            .await?
            .ok_or(ClientError::TooEarly("sum dict"))?;

        let upd_msg =
            self.participant
                .compose_update_message(self.round_params.pk, &sums, local_model);
        let sealed_msg = self
            .participant
            .seal_message(&self.round_params.pk, &upd_msg);

        debug!("sending update message");
        api.send_message(sealed_msg).await?;
        info!("update participant completed a round");
        Ok(())
    }
}

impl ClientState<Sum2> {
    fn new(participant: Participant<Sum2>, round_params: RoundParameters) -> Self {
        Self {
            participant,
            round_params,
        }
    }

    async fn next<T: ApiClient>(mut self, api: &mut T) -> ClientStateMachine {
        info!("selected to sum2");

        match self.run(api).await {
            Ok(_) | Err(ClientError::RoundOutdated) => self.reset().into(),
            Err(err) => {
                error!("{:?}", err);
                self.into()
            }
        }
    }

    async fn run<T: ApiClient>(&mut self, api: &mut T) -> Result<(), ClientError<T::Error>> {
        self.check_round_freshness(api).await?;

        debug!("polling for model/mask length");
        let length = api
            .get_mask_length()
            .await?
            .ok_or(ClientError::TooEarly("length"))?;
        if length > usize::MAX as u64 {
            return Err(ClientError::ParticipantErr(PetError::InvalidModel));
        };

        debug!("polling for seed dict");
        let seeds = api
            .get_seeds(self.participant.get_participant_pk())
            .await?
            .ok_or(ClientError::TooEarly("seeds"))?;

        let sum2_msg = self
            .participant
            .compose_sum2_message(self.round_params.pk, &seeds, length as usize)
            .map_err(|e| {
                error!("failed to compose sum2 message with seeds: {:?}", &seeds);
                ClientError::ParticipantErr(e)
            })?;
        let sealed_msg = self
            .participant
            .seal_message(&self.round_params.pk, &sum2_msg);

        debug!("sending sum2 message");
        api.send_message(sealed_msg).await?;
        info!("sum participant completed a round");
        Ok(())
    }
}

#[derive(From, Serialize, Deserialize)]
pub enum ClientStateMachine {
    Awaiting(ClientState<Awaiting>),
    Sum(ClientState<Sum>),
    Update(ClientState<Update>),
    Sum2(ClientState<Sum2>),
}

impl ClientStateMachine {
    pub fn new(participant_settings: ParticipantSettings) -> Result<Self, InitError> {
        // crucial: init must be called before anything else in this module
        sodiumoxide::init().or(Err(InitError))?;

        Ok(ClientState::<Awaiting>::new(
            Participant::<Awaiting>::new(participant_settings.into()),
            RoundParameters::default(),
        )
        .into())
    }

    pub async fn next<L: LocalModel, T: ApiClient>(self, api: &mut T, local_model: &mut L) -> Self {
        match self {
            ClientStateMachine::Awaiting(state) => state.next(api).await,
            ClientStateMachine::Sum(state) => state.next(api).await,
            ClientStateMachine::Update(state) => state.next(api, local_model).await,
            ClientStateMachine::Sum2(state) => state.next(api).await,
        }
    }
}
