use super::{CoordinatorState, State, StateError, StateMachine};
use crate::{
    coordinator_async::{
        error::Error,
        message::{MessageHandler, MessageSink, UpdateValidationData},
        sum2::Sum2,
    },
    message::{MessageOwned, PayloadOwned},
    PetError,
};
use std::{default::Default, future::Future, pin::Pin, sync::Arc};
use tokio::{
    sync::{broadcast, mpsc},
    time::Duration,
};

pub struct Update {
    update_validation_data: Arc<UpdateValidationData>,
}

impl State<Update> {
    pub fn new(
        coordinator_state: CoordinatorState,
        message_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    ) -> StateMachine {
        let update_validation_data = Arc::new(UpdateValidationData {
            seed: coordinator_state.seed.clone(),
            sum: coordinator_state.sum,
            update: coordinator_state.update,
        });

        StateMachine::Update(Self {
            _inner: Update {
                update_validation_data,
            },
            coordinator_state,
            message_rx,
        })
    }

    pub async fn next(mut self) -> StateMachine {
        info!("Update phase!");
        match self.run().await {
            Ok(_) => State::<Sum2>::new(self.coordinator_state, self.message_rx),
            Err(err) => State::<Error>::new(self.coordinator_state, self.message_rx, err),
        }
    }

    async fn run(&mut self) -> Result<(), StateError> {
        let (sink_tx, sink) =
            MessageSink::new(10, Duration::from_secs(5), Duration::from_secs(1000));
        let (_cancel_complete_tx, mut cancel_complete_rx) = mpsc::channel::<()>(1);
        let (notify_cancel, _) = broadcast::channel::<()>(1);

        let phase_result = tokio::select! {
            message_source_result = async {

                loop {
                    let message = self.next_message().await?;

                    let message_handler = self.create_message_handler(
                        message, sink_tx.clone(),
                        _cancel_complete_tx.clone(),
                        notify_cancel.subscribe(),
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
    ) -> Result<Pin<Box<dyn Future<Output = ()> + 'static + Send>>, PetError> {
        let participant_pk = message.header.participant_pk;
        let update_message = match message.payload {
            PayloadOwned::Update(msg) => msg,
            _ => return Err(PetError::InvalidMessage),
        };

        let message_handler =
            MessageHandler::new(sink_tx.clone(), _cancel_complete_tx.clone(), notify_cancel);

        Ok(Box::pin(message_handler.handle_update_message(
            self._inner.update_validation_data.clone(),
            participant_pk,
            update_message,
        )))
    }
}
