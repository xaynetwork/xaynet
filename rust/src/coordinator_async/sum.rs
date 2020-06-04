use super::{State, StateMachine};
use crate::{
    coordinator_async::{
        error::Error,
        message_processing::{MessageHandler, MessageSink, SumValidationData},
        update::Update,
    },
    message::{MessageOwned, PayloadOwned},
    PetError,
};
use std::{collections::HashMap, default::Default, future::Future, pin::Pin, sync::Arc};
use tokio::{
    sync::{broadcast, mpsc},
    time::Duration,
};

#[derive(Debug)]
pub struct Sum;

impl State<Sum> {
    pub async fn next(mut self) -> StateMachine {
        println!("Sum phase!");

        match self.run().await {
            Ok(_) => {
                // Fetch sum dict?
                let sum_dict = HashMap::new();
                StateMachine::Update(State {
                    _inner: Update {
                        sum_dict: Some(Arc::new(sum_dict)),
                    },
                    coordinator_state: self.coordinator_state,
                    message_rx: self.message_rx,
                })
            }

            Err(_) => StateMachine::Error(State {
                _inner: Error {},
                coordinator_state: self.coordinator_state,
                message_rx: self.message_rx,
            }),
        }
    }

    async fn run(&mut self) -> Result<(), PetError> {
        let (sink_tx, sink) =
            MessageSink::new(10, Duration::from_secs(5), Duration::from_secs(1000));
        let (_cancel_complete_tx, mut cancel_complete_rx) = mpsc::channel::<()>(1);
        let (notify_cancel, _) = broadcast::channel::<()>(1);

        let phase_result = tokio::select! {
            message_source_result = async {
                let sum_validation_data = Arc::new(SumValidationData {
                    seed: self.coordinator_state.seed.clone(),
                    sum: self.coordinator_state.sum,
                });

                loop {
                    let message = self.next_message().await?;
                    let message_handler = self.create_message_handler(
                        message, sink_tx.clone(),
                        _cancel_complete_tx.clone(),
                        notify_cancel.subscribe(),
                        sum_validation_data.clone()
                    )?;
                    tokio::spawn(async move { message_handler.await });
                }
            } => {
                message_source_result
            }
            message_sink_result = sink.collect() => {
                message_sink_result
            }
        };

        // Drop the notify_cancel sender. By dropping the sender, all receivers will receive a
        // RecvError.
        drop(notify_cancel);

        // Wait until all MessageHandler tasks have been resolved/canceled.
        // (After all senders of this channel are dropped, which mean that all
        // MessageHandler have been dropped, the receiver of this channel will receive None).
        drop(_cancel_complete_tx);
        let _ = cancel_complete_rx.recv().await;

        phase_result
    }

    fn create_message_handler(
        &mut self,
        message: MessageOwned,
        sink_tx: mpsc::UnboundedSender<Result<(), PetError>>,
        _cancel_complete_tx: mpsc::Sender<()>,
        notify_cancel: broadcast::Receiver<()>,
        sum_validation_data: Arc<SumValidationData>,
    ) -> Result<Pin<Box<dyn Future<Output = ()> + 'static + Send>>, PetError> {
        let participant_pk = message.header.participant_pk;
        let sum_message = match message.payload {
            PayloadOwned::Sum(msg) => msg,
            _ => return Err(PetError::InvalidMessage),
        };

        let message_handler =
            MessageHandler::new(sink_tx.clone(), _cancel_complete_tx.clone(), notify_cancel);

        Ok(Box::pin(message_handler.handle_sum_message(
            sum_validation_data,
            participant_pk,
            sum_message,
        )))
    }
}
