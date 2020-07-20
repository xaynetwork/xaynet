use crate::{
    client::{client::ClientStateMachine, participant_::ParticipantSettings, Proxy},
    mask::model::Model,
};
use std::{cell::RefCell, rc::Rc};

pub struct MobileClient {
    runtime: tokio::runtime::Runtime,
    local_model: Rc<RefCell<Option<Model>>>,
    global_model: Rc<RefCell<Option<Model>>>,
    client_state: Option<ClientStateMachine>,
}

impl MobileClient {
    pub fn new(url: &str, participant_settings: ParticipantSettings) -> Self {
        let runtime = tokio::runtime::Builder::new()
            .basic_scheduler()
            .enable_all()
            .build()
            .unwrap();

        let local_model = Rc::new(RefCell::new(None));
        let global_model = Rc::new(RefCell::new(None));

        let client_state = ClientStateMachine::new(
            Proxy::new_remote(url),
            participant_settings,
            local_model.clone(),
            global_model.clone(),
        )
        .unwrap();

        Self {
            runtime,
            local_model,
            global_model,
            client_state: Some(client_state),
        }
    }

    pub fn set_local_model(&self, local_model: Model) {
        *self.local_model.borrow_mut() = Some(local_model);
    }

    pub fn get_global_model(&self) -> Option<Model> {
        self.global_model.borrow().clone()
    }

    pub fn next(&mut self) {
        if let Some(current_state) = self.client_state.take() {
            let new_state = self
                .runtime
                .block_on(async move { current_state.next().await });
            self.client_state = Some(new_state);
        }
    }
}
