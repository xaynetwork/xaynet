use futures::executor::block_on;
use std::thread;
use tokio::{
    select,
    sync::{
        mpsc::{
            unbounded_channel as channel, UnboundedReceiver as Receiver, UnboundedSender as Sender,
        },
        oneshot,
    },
};
// FIXME: the code should be loaded from a file. This is just an
// example to get going.
static CODE: &'static str = r#"
from typing import Optional
import bz2
import numpy as np
import pickle

DUMMY_WEIGHTS = np.ndarray([1,2,3])

class Aggregator:

    def __init__(self):
        self.global_weights = DUMMY_WEIGHTS
        self.weights = []

    def add_weights(self, data: bytes) -> bool:
        weights = pickle.loads(bz2.decompress(data))
        self.weights.append(weights)
        return True

    def aggregate(self) -> bytes:
        # Do nothing for now, just return the global weights
        data = bz2.compress(pickle.dumps(self.global_weights))
        return data

    def reset(self, global_weights: Optional[np.ndarray]) -> None:
        if global_weights is None: global_weights = DUMMY_WEIGHTS self.weights = []

    def get_global_weights(self) -> np.ndarray:
        data = bz2.compress(pickle.dumps(self.global_weights))
        return data
"#;

use pyo3::{
    types::{PyBytes, PyModule},
    PyObject, PyResult, Python, ToPyObject,
};

pub struct PyAggregator<'py> {
    py: Python<'py>,
    aggregator: PyObject,
}

impl<'py> PyAggregator<'py> {
    pub fn load(py: Python<'py>) -> PyResult<Self> {
        let module = PyModule::from_code(py, CODE, "aggregator.py", "aggregator")?;
        let aggregator = module.call0("Aggregator")?.to_object(py);
        Ok(Self { py, aggregator })
    }

    pub fn aggregate(&self) -> PyResult<Vec<u8>> {
        Ok(self
            .aggregator
            .call_method0(self.py, "aggregate")?
            .extract(self.py)?)
    }

    pub fn get_global_weights(&self) -> PyResult<Vec<u8>> {
        Ok(self
            .aggregator
            .call_method0(self.py, "get_global_weights")?
            .extract(self.py)?)
    }

    pub fn add_weights(&self, local_weights: &[u8]) -> PyResult<Result<(), ()>> {
        let py_bytes = PyBytes::new(self.py, local_weights);
        let args = (py_bytes,);
        Ok(self
            .aggregator
            .call_method1(self.py, "add_weights", args)?
            .extract::<bool>(self.py)?
            .then_some(())
            .ok_or(()))
    }

    pub fn reset(&self, global_weights: &[u8]) -> PyResult<()> {
        let py_bytes = PyBytes::new(self.py, global_weights);
        let args = (py_bytes,);
        self.aggregator.call_method1(self.py, "reset", args)?;
        Ok(())
    }
}

pub type Weights = Vec<u8>;
pub type Request<T, U> = (T, oneshot::Sender<U>);
pub type RequestRx<T, U> = Receiver<Request<T, U>>;
pub type RequestTx<T, U> = Sender<Request<T, U>>;

pub fn spawn_py_aggregator() -> PyAggregatorHandle {
    let (aggregate_tx, aggregate_rx) = channel::<Request<(), Weights>>();
    let (add_weights_tx, add_weights_rx) = channel::<Request<Weights, ()>>();
    thread::spawn(move || block_on(py_aggregator(aggregate_rx, add_weights_rx)));
    PyAggregatorHandle {
        aggregate_requests: aggregate_tx,
        add_weights_requests: add_weights_tx,
    }
}

pub struct PyAggregatorHandle {
    pub aggregate_requests: RequestTx<(), Weights>,
    pub add_weights_requests: RequestTx<Weights, ()>,
}

async fn py_aggregator(
    mut aggregate_requests: RequestRx<(), Weights>,
    mut add_weights_requests: RequestRx<Weights, ()>,
) {
    let gil = Python::acquire_gil();
    let py = gil.python();
    let aggregator = PyAggregator::load(py).unwrap();

    loop {
        select! {
            Some(((), resp_tx)) = aggregate_requests.recv() => {
                let weights = aggregator.aggregate().unwrap();
                if resp_tx.send(weights).is_err() {
                    warn!("cannot send aggregate response, receiver has been dropped");
                    return;
                }

            }
            Some((weights, resp_tx)) = add_weights_requests.recv() => {
                aggregator.add_weights(&weights[..]).unwrap();
                if resp_tx.send(()).is_err() {
                    warn!("cannot send add_weights response, receiver has been dropped");
                    return;
                }
            }
            else => {
                warn!("one of the PyAggregator receivers was dropped");
                return;
            }
        }
    }
}
