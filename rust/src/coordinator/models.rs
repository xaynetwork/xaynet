use crate::common::{ClientId, Token};

/// Response to a heartbeat
#[derive(Debug)]
pub enum HeartBeatResponse {
    /// The client should stand by in its current state
    StandBy,

    /// The coordinator has finished, and the client should disconnect
    Finish,

    /// The client has been selected for the given round and should
    /// start or continue training
    Round(u32),

    /// The client has not been accepted by the coordinator yet and
    /// should not send heartbeats
    Reject,
}

#[derive(Debug)]
pub enum RendezVousResponse {
    Accept(ClientId),
    Reject,
}

#[derive(Debug)]
pub enum StartTrainingResponse {
    Accept(String, Token),
    Reject,
}

pub mod json {
    use super::*;

    mod rendez_vous {
        use super::RendezVousResponse;
        use crate::common::ClientId;

        #[derive(Serialize)]
        pub struct RendezVousResponseJson {
            id: Option<ClientId>,
            ok: bool,
        }

        impl From<RendezVousResponse> for RendezVousResponseJson {
            fn from(resp: RendezVousResponse) -> Self {
                use RendezVousResponse::*;
                match resp {
                    Accept(id) => Self {
                        ok: true,
                        id: Some(id),
                    },
                    Reject => Self {
                        ok: false,
                        id: None,
                    },
                }
            }
        }
    }

    mod heartbeat {
        use super::HeartBeatResponse;
        #[derive(Serialize)]
        #[serde(rename_all = "snake_case")]
        enum State {
            StandBy,
            Finish,
            Round,
            Reject,
        }

        #[derive(Serialize)]
        pub struct HeartBeatResponseJson {
            round: Option<u32>,
            state: State,
        }

        impl From<HeartBeatResponse> for HeartBeatResponseJson {
            fn from(resp: HeartBeatResponse) -> Self {
                use HeartBeatResponse::*;
                match resp {
                    StandBy => Self {
                        round: None,
                        state: State::StandBy,
                    },
                    Finish => Self {
                        round: None,
                        state: State::Finish,
                    },
                    Round(round) => Self {
                        round: Some(round),
                        state: State::Round,
                    },
                    Reject => Self {
                        round: None,
                        state: State::Reject,
                    },
                }
            }
        }
    }

    mod start_training {
        use super::StartTrainingResponse;
        use crate::common::Token;

        #[derive(Serialize)]
        pub struct StartTrainingResponseJson {
            url: Option<String>,
            token: Option<Token>,
            ok: bool,
        }

        impl From<StartTrainingResponse> for StartTrainingResponseJson {
            fn from(resp: StartTrainingResponse) -> Self {
                use StartTrainingResponse::*;
                match resp {
                    Accept(url, token) => Self {
                        ok: true,
                        url: Some(url),
                        token: Some(token),
                    },
                    Reject => Self {
                        ok: false,
                        url: None,
                        token: None,
                    },
                }
            }
        }
    }

    pub use heartbeat::*;
    pub use rendez_vous::*;
    pub use start_training::*;
}
