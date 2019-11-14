use futures_channel::mpsc;

use std::time::Duration;
use xain_coordinator::training::{
    InMessage, OutMessage, TimeoutToken, Training, TrainingParams, IO,
};

use async_std::prelude::*;
use ndarray::array;
use ndarray::prelude::*;

type Sender<T> = mpsc::UnboundedSender<T>;
type Receiver<T> = mpsc::UnboundedReceiver<T>;

pub struct TrainingTask {
    receiver: Receiver<InMessage>,
    training: Training,
}

struct NullIO;

impl IO for NullIO {
    fn send(&mut self, msg: OutMessage) {
        println!("Outmessage: {:?}", msg);
        // ignore
    }
    fn schedule_timeout(&mut self, _duration: Duration) -> TimeoutToken {
        println!("Timeout Scheduled");
        TimeoutToken { on_cancel: Box::new(()) }
        // ignore
    }
}

impl TrainingTask {
    pub fn create() -> (TrainingTask, Sender<InMessage>) {
        let (sender, receiver) = mpsc::unbounded();

        let initial_model = vec![array![0.0, 0.0].into_dyn()];
        let training = Training::new(TrainingParams {
            model_dim: vec![IxDyn(&[2])],
            initial_model: initial_model.clone(),
            n_participants: 1,
            n_rounds: 1,
            round_timeout: Duration::from_secs(10),
        });

        (TrainingTask { training, receiver }, sender)
    }

    pub async fn run(&mut self) {
        while let Some(message) = self.receiver.next().await {
            println!("Incoming Message: {:?}", message);
            self.training.on_message(message, &mut NullIO).unwrap();
        }
    }
}
