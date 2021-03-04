//! In this file `Sender` is just stubbed and will need to be implemented.

use anyhow::{Error, Result};

use crate::data_combination::data_points::data_point::DataPoint;

/// `Sender` receives a `Vec<DataPoint>` from the `DataCombiner`.
///
/// It will need to call the exposed `calculate()` method on each `DataPoint` variant and compose the messages
/// that will then need to reach the XayNet coordinator.
///
/// These messages should contain not only the actual data that is the output of calling `calculate()` on the variant,
/// but also some extra data so that the coordinator knows how to aggregate each `DataPoint` variant.
/// This is in line with the research done on the “global spec” idea.
pub struct Sender;

impl Sender {
    pub fn send(&self, _data_points: Vec<DataPoint>) -> Result<(), Error> {
        // TODO: https://xainag.atlassian.net/browse/XN-1647
        todo!()
    }
}
