use anyhow::{Error, Result};

use crate::data_combination::data_points::data_point::DataPoint;

pub struct Sender;

impl<'ctrl> Sender {
    pub fn send(&self, _data_points: Vec<DataPoint<'ctrl>>) -> Result<(), Error> {
        // TODO: https://xainag.atlassian.net/browse/XN-1647
        todo!()
    }
}
