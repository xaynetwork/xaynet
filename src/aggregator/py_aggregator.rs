// FIXME: the code should be loaded from a file
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
        if global_weights is None:
            global_weights = DUMMY_WEIGHTS
        self.weights = []

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
