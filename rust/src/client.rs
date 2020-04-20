use extern crate::service::Handle;
use extern crate::participant::Participant;

/// TODO
pub struct Client {
    handle: Handle,
    participant: Participant,
}

impl Client {
    /// Create a new `Client`
    pub fn new() -> Self {
        let (handle, _events) = Handle::new();
        let participant = Participant::new();
        Self {
            handle,
            participant,
        }
    }

    /// TODO
    pub fn start(&mut self) {
        self.pre_round()
    }

    fn pre_round(&mut self) {
        // TODO
        match self.handle.get_round_parameters().await {
            Some(round_params) => panic!(),
            None => panic!(),
        }
    }

    fn unselected(&mut self) {
        // TODO
    }

    fn summer(&mut self) {
        // TODO
    }

    fn updater(&mut self) {
        // TODO
    }
}
