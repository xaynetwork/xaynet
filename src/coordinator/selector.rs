use crate::common::ClientId;

pub trait Selector {
    fn select(
        &mut self,
        min_count: usize,
        waiting: impl Iterator<Item = ClientId>,
        selected: impl Iterator<Item = ClientId>,
    ) -> Vec<ClientId>;
}
