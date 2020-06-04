use super::{State, StateMachine};
use crate::{
    coordinator_async::{
        error::Error,
        message_processing::{MessageHandler, MessageSink, UpdateValidationData},
        sum2::Sum2,
    },
    message::{MessageOwned, PayloadOwned},
    PetError,
    SumDict,
};
use std::{default::Default, future::Future, pin::Pin, sync::Arc};
use tokio::{
    sync::{broadcast, mpsc},
    time::Duration,
};

#[derive(Debug)]
pub struct Update {
    pub sum_dict: Option<Arc<SumDict>>,
}
impl State<Update> {
    pub async fn next(mut self) -> StateMachine {
        println!("Update phase!");
        match self.run().await {
            Ok(_) => StateMachine::Sum2(State {
                _inner: Sum2 {},
                coordinator_state: self.coordinator_state,
                message_rx: self.message_rx,
            }),
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
                let update_validation_data = Arc::new(UpdateValidationData {
                    seed: self.coordinator_state.seed.clone(),
                    sum: self.coordinator_state.sum,
                    update: self.coordinator_state.update,
                });

                loop {
                    let message = self.next_message().await?;

                    let message_handler = self.create_message_handler(
                        message, sink_tx.clone(),
                        _cancel_complete_tx.clone(),
                        notify_cancel.subscribe(),
                        update_validation_data.clone()
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

        drop(notify_cancel);
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
        update_validation_data: Arc<UpdateValidationData>,
    ) -> Result<Pin<Box<dyn Future<Output = ()> + 'static + Send>>, PetError> {
        let participant_pk = message.header.participant_pk;
        let update_message = match message.payload {
            PayloadOwned::Update(msg) => msg,
            _ => return Err(PetError::InvalidMessage),
        };

        let message_handler =
            MessageHandler::new(sink_tx.clone(), _cancel_complete_tx.clone(), notify_cancel);

        Ok(Box::pin(message_handler.handle_update_message(
            update_validation_data,
            participant_pk,
            update_message,
        )))
    }
}
