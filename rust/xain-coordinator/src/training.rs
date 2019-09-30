use std::{
    any::Any,
    collections::{BTreeMap, BTreeSet},
    fmt,
    time::Duration,
};

use crate::{Model, ModelDim};

macro_rules! err {
    ($($tt:tt)*) => {
        ProtocolError { msg: format!($($tt)*) }
    }
}

macro_rules! bail {
    ($($tt:tt)*) => {
        return Err(err!($($tt)*))
    }
}

#[derive(Debug)]
pub struct ProtocolError {
    msg: String,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.msg, f)
    }
}

impl std::error::Error for ProtocolError {}

type ParticipantId = u32;

#[derive(Debug)]
pub struct FromParticipant<T> {
    pub from: ParticipantId,
    pub payload: T,
}

#[derive(Debug)]
pub enum InMessage {
    Joined(FromParticipant<()>),
    Update(FromParticipant<Model>),
    RoundTimeout,
}

impl InMessage {
    pub fn new_joined(from: ParticipantId) -> InMessage {
        InMessage::Joined(FromParticipant { from, payload: () })
    }

    pub fn new_update(from: ParticipantId, model: Model) -> InMessage {
        InMessage::Update(FromParticipant { from, payload: model })
    }

    pub fn new_round_timeout() -> InMessage {
        InMessage::RoundTimeout
    }
}

#[derive(Debug)]
pub struct OutMessage {
    pub to: ParticipantId,
    pub kind: OutMessageKind,
}

#[derive(Debug)]
pub enum OutMessageKind {
    Update { model: Model },
    Finished { model: Model },
}

pub trait IO {
    fn send(&mut self, msg: OutMessage);
    fn schedule_timeout(&mut self, duration: Duration) -> TimeoutToken;
}

/// Dropping `TimeoutToken` cancels the associated timeout.
pub struct TimeoutToken {
    pub on_cancel: Box<dyn Any>,
}

pub struct TrainingParams {
    model_dim: ModelDim,
    initial_model: Model,
    n_participants: u32,
    n_rounds: u32,
    round_timeout: Duration,
}

pub struct Training {
    params: TrainingParams,
    state: TrainingState,
}

enum TrainingState {
    Joining { participants: BTreeSet<ParticipantId> },
    Round(RoundState),
    Finished,
}

struct RoundState {
    participants: BTreeSet<ParticipantId>,
    round_results: BTreeMap<ParticipantId, Model>,
    model: Model,
    round: u32,
    timeout_token: Option<TimeoutToken>,
}

impl Training {
    pub fn new(params: TrainingParams) -> Training {
        Training { params, state: TrainingState::Joining { participants: BTreeSet::new() } }
    }

    pub fn on_message(&mut self, message: InMessage, io: &mut dyn IO) -> Result<(), ProtocolError> {
        log::info!("on_message({:?})", message);
        match (&mut self.state, &message) {
            (TrainingState::Joining { participants }, InMessage::Joined(joined)) => {
                let is_new = participants.insert(joined.from);
                if !is_new {
                    bail!("peer joined twice: {}", joined.from)
                }
                if participants.len() == self.params.n_participants as usize {
                    let mut round =
                        RoundState::new(take(participants), self.params.initial_model.clone());
                    round.start_round(io, self.params.round_timeout);
                    self.state = TrainingState::Round(round)
                }
            }
            (TrainingState::Joining { .. }, _) => bail!("unexpected message in Joining state"),

            (TrainingState::Round(round), InMessage::Update(update)) => {
                check_dim(&update.payload, &self.params.model_dim)?;
                round.update(update.from, update.payload.clone())?;
                if round.is_completed() {
                    round.move_to_next_round();
                    if round.round == self.params.n_rounds {
                        let final_model = take(&mut round.model);
                        for &participant in round.participants.iter() {
                            io.send(OutMessage {
                                to: participant,
                                kind: OutMessageKind::Finished { model: final_model.clone() },
                            });
                        }
                        self.state = TrainingState::Finished
                    } else {
                        round.start_round(io, self.params.round_timeout);
                    }
                }
            }
            (TrainingState::Round(_), InMessage::RoundTimeout) => bail!("round timed out"),
            (TrainingState::Round { .. }, _) => bail!("unexpected message in Joining state"),

            (TrainingState::Finished { .. }, InMessage::RoundTimeout) => {}
            (TrainingState::Finished { .. }, _) => bail!("unexpected message in Finished state"),
        }
        Ok(())
    }
}

impl RoundState {
    fn new(participants: BTreeSet<ParticipantId>, model: Model) -> RoundState {
        RoundState {
            participants,
            round_results: BTreeMap::new(),
            model: model,
            round: 0,
            timeout_token: None,
        }
    }

    fn update(&mut self, id: ParticipantId, model: Model) -> Result<(), ProtocolError> {
        if !self.selected_participants().contains(&id) {
            bail!("unselected participant")
        }
        if let Some(_previous) = self.round_results.insert(id, model) {
            bail!("participant send a model twice")
        }
        Ok(())
    }

    fn start_round(&mut self, io: &mut dyn IO, timeout: Duration) {
        for &to in self.selected_participants().iter() {
            io.send(OutMessage { to, kind: OutMessageKind::Update { model: self.model.clone() } })
        }
        self.timeout_token = Some(io.schedule_timeout(timeout));
    }

    fn selected_participants(&self) -> &BTreeSet<ParticipantId> {
        &self.participants
    }

    fn move_to_next_round(&mut self) {
        let models = self.round_results.iter().map(|(_, value)| value.clone()).collect::<Vec<_>>();
        self.round_results.clear();
        let weights = (0..models.len()).map(|_| 1).collect::<Vec<_>>();
        self.model = crate::aggregation::federated_average(&models, &weights);
        self.round += 1;
    }

    fn is_completed(&self) -> bool {
        self.round_results.len() == self.selected_participants().len()
    }
}

fn check_dim(model: &Model, model_dim: &ModelDim) -> Result<(), ProtocolError> {
    if !(model.len() == model_dim.len() && model.iter().zip(model_dim).all(|(t, d)| t.dim() == *d))
    {
        bail!("invalid model dimensions")
    }
    Ok(())
}

// Replace with std::mem::take once it is stable
fn take<T: Default>(slot: &mut T) -> T {
    std::mem::replace(slot, T::default())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use ndarray::array;
    use ndarray::prelude::*;

    use super::*;

    #[derive(Default)]
    struct Mailbox {
        messages: Vec<OutMessage>,
        timeout: Arc<Mutex<Option<Duration>>>,
    }

    impl IO for Mailbox {
        fn send(&mut self, msg: OutMessage) {
            self.messages.push(msg);
        }
        fn schedule_timeout(&mut self, duration: Duration) -> TimeoutToken {
            let timeout = Arc::clone(&self.timeout);
            *timeout.lock().unwrap() = Some(duration);

            struct CancelTimeout(Arc<Mutex<Option<Duration>>>);
            impl Drop for CancelTimeout {
                fn drop(&mut self) {
                    self.0.lock().unwrap().take();
                }
            }

            TimeoutToken { on_cancel: Box::new(CancelTimeout(timeout)) }
        }
    }

    impl Mailbox {
        fn is_empty(&self) -> bool {
            self.messages.is_empty()
        }

        fn drain(&mut self, expected: usize) -> std::vec::Drain<OutMessage> {
            assert_eq!(self.messages.len(), expected);
            self.messages.drain(..)
        }

        fn timeout(&self) -> Option<Duration> {
            *self.timeout.lock().unwrap()
        }
    }

    #[test]
    fn smoke_test() {
        let initial_model = vec![array![0.0, 0.0].into_dyn()];
        let mut training = Training::new(TrainingParams {
            model_dim: vec![IxDyn(&[2])],
            initial_model: initial_model.clone(),
            n_participants: 2,
            n_rounds: 1,
            round_timeout: Duration::from_secs(10),
        });
        let mut mailbox = Mailbox::default();

        training.on_message(InMessage::new_joined(0), &mut mailbox).unwrap();
        assert!(mailbox.is_empty());

        training.on_message(InMessage::new_joined(1), &mut mailbox).unwrap();
        for m in mailbox.drain(2) {
            match m.kind {
                OutMessageKind::Update { model } => assert_eq!(model, initial_model),
                _ => panic!(),
            }
        }

        training
            .on_message(InMessage::new_update(0, vec![array![1.0, 0.0].into_dyn()]), &mut mailbox)
            .unwrap();
        assert!(mailbox.is_empty());

        training
            .on_message(InMessage::new_update(1, vec![array![0.0, 1.0].into_dyn()]), &mut mailbox)
            .unwrap();

        for m in mailbox.drain(2) {
            match m.kind {
                OutMessageKind::Finished { model } => {
                    assert_eq!(model, vec![array![0.5, 0.5].into_dyn()])
                }
                _ => panic!(),
            }
        }
    }

    #[test]
    fn test_round_timeout() {
        let initial_model = vec![array![0.0, 0.0].into_dyn()];
        let mut training = Training::new(TrainingParams {
            model_dim: vec![IxDyn(&[2])],
            initial_model: initial_model.clone(),
            n_participants: 2,
            n_rounds: 1,
            round_timeout: Duration::from_secs(10),
        });
        let mut mailbox = Mailbox::default();

        for &participant in [0, 1].iter() {
            training.on_message(InMessage::new_joined(participant), &mut mailbox).unwrap();
        }
        mailbox.drain(2);
        assert_eq!(mailbox.timeout(), Some(Duration::from_secs(10)));

        let res = training.on_message(InMessage::new_round_timeout(), &mut mailbox);
        match res {
            Ok(_) => panic!(),
            Err(e) => assert_eq!(e.to_string().as_str(), "round timed out"),
        }
    }
}
