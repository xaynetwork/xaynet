use std::{collections::HashMap, error::Error, marker::PhantomData};

use crate::{
    common::{ClientId, Token},
    coordinator::CoordinatorTarpcClient,
};
use serde::Serialize;

struct AggregatorService<A, W>
where
    A: Aggregator<W>,
    W: Serialize,
{
    known_ids: HashMap<ClientId, Token>,
    global_weights: Vec<u8>,
    aggregator: A,
    rpc_client: CoordinatorTarpcClient,
    _phantom: PhantomData<W>,
}

trait Aggregator<W>
where
    W: Serialize,
{
    type Error: Error;

    fn add_weights(&mut self, weights: W) -> Result<(), Self::Error>;
    fn aggregate(&mut self) -> Result<W, Self::Error>;
}
