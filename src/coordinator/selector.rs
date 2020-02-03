use crate::coordinator::client::ClientId;
use std::collections::HashSet;

pub trait Selector {
    fn select(
        &mut self,
        nodes: &mut HashSet<ClientId>,
        active_participants: &mut HashSet<ClientId>,
        done_participants: &HashSet<ClientId>,
        fraction: f32,
        round: u32,
    );
}
