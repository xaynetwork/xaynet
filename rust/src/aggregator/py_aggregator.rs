use bytes::Bytes;
use futures::executor::block_on;
use std::{error::Error, future::Future, pin::Pin, thread};
use thiserror::Error;
use tokio::{
    select,
    sync::{
        mpsc::{channel, unbounded_channel, Receiver, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
};

use crate::aggregator::{service::Aggregator, settings::PythonAggregatorSettings};
use pyo3::{
    types::{PyBytes, PyModule},
    GILGuard, PyObject, PyResult, Python, ToPyObject,
};

pub struct PyAggregator {
    gil: Option<GILGuard>,
    aggregator: PyObject,
}

impl PyAggregator {
    pub fn load(settings: PythonAggregatorSettings) -> Result<Self, PythonError> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let module = PyModule::import(py, &settings.module).map_err(|e| {
            // Currently, there is no easy way to convert `PyErr` into
            // a Rust error type so we just print the error on
            // stderr. See: https://github.com/PyO3/pyo3/issues/592
            // and https://github.com/PyO3/pyo3/issues/682
            e.print(py);
            PythonError::LoadModule(settings.module.clone())
        })?;
        let aggregator = module
            .call0(&settings.class)
            .map_err(|e| {
                // Currently, there is no easy way to convert `PyErr` into
                // a Rust error type so we just print the error on
                // stderr. See: https://github.com/PyO3/pyo3/issues/592
                // and https://github.com/PyO3/pyo3/issues/682
                e.print(py);
                PythonError::LoadClass(settings.module.clone(), settings.class.clone())
            })?
            .to_object(py);
        Ok(Self {
            gil: Some(gil),
            aggregator,
        })
    }

    pub fn aggregate(&mut self) -> Result<Bytes, PyAggregatorError<AggregationFailed>> {
        info!("running aggregation");
        let py = self.get_py();
        let result = self
            .aggregator
            .call_method0(py, "aggregate")
            .map_err(|e| {
                // Currently, there is no easy way to convert `PyErr` into
                // a Rust error type so we just print the error on
                // stderr. See: https://github.com/PyO3/pyo3/issues/592
                // and https://github.com/PyO3/pyo3/issues/682
                e.print(py);
                PyAggregatorError::<AggregationFailed>::from(PythonError::Call("aggregate"))
            })?
            .extract::<Vec<u8>>(py)
            .map(Bytes::from)
            .map_err(|e| {
                // Currently, there is no easy way to convert `PyErr` into
                // a Rust error type so we just print the error on
                // stderr. See: https://github.com/PyO3/pyo3/issues/592
                // and https://github.com/PyO3/pyo3/issues/682
                e.print(py);
                PythonError::Cast("bytes", "Vec<u8>")
            })?;
        info!("finished aggregation");
        self.re_acquire_gil();
        Ok(result)
    }

    /// Release the GIL so that python's garbage collector runs
    fn re_acquire_gil(&mut self) {
        self.gil = None;
        self.gil = Some(Python::acquire_gil());
    }

    pub fn get_global_weights(&self) -> PyResult<Bytes> {
        let py = self.get_py();
        Ok(self
            .aggregator
            .call_method0(py, "get_global_weights")?
            .extract::<Vec<u8>>(py)
            .map(Bytes::from)?)
    }

    pub fn add_weights(
        &self,
        local_weights: &[u8],
    ) -> Result<(), PyAggregatorError<InvalidWeights>> {
        info!("adding weights");
        let py = self.get_py();
        let py_bytes = PyBytes::new(py, local_weights);
        let args = (py_bytes,);
        self.aggregator
            .call_method1(py, "add_weights", args)
            .map_err(|e| {
                // Currently, there is no easy way to convert `PyErr` into
                // a Rust error type so we just print the error on
                // stderr. See: https://github.com/PyO3/pyo3/issues/592
                // and https://github.com/PyO3/pyo3/issues/682
                e.print(py);
                PyAggregatorError::<InvalidWeights>::from(PythonError::Call("add_weights"))
            })?
            .extract::<bool>(py)
            .map_err(|e| {
                // Currently, there is no easy way to convert `PyErr` into
                // a Rust error type so we just print the error on
                // stderr. See: https://github.com/PyO3/pyo3/issues/592
                // and https://github.com/PyO3/pyo3/issues/682
                e.print(py);
                PyAggregatorError::from(PythonError::Cast("bool", "bool"))
            })?
            .then_some(())
            .ok_or(PyAggregatorError::Request(InvalidWeights))?;
        info!("Done adding weights");
        Ok(())
    }

    pub fn get_py(&self) -> Python<'_> {
        // UNWRAP_SAFE: As long as PyAggregator exists, self.gil
        // cannot be None: the only place where we temporarily set it
        // to None is in PyAggregator.re_acquire_gil(), but we set it
        // back to Some right away.
        self.gil.as_ref().unwrap().python()
    }

    pub fn reset(&mut self, global_weights: &[u8]) -> Result<(), PythonError> {
        info!("resetting weights");
        let py = self.get_py();
        let py_bytes = PyBytes::new(py, global_weights);
        let args = (py_bytes,);
        self.aggregator
            .call_method1(py, "reset", args)
            .map_err(|e| {
                // Currently, there is no easy way to convert `PyErr` into
                // a Rust error type so we just print the error on
                // stderr. See: https://github.com/PyO3/pyo3/issues/592
                // and https://github.com/PyO3/pyo3/issues/682
                e.print(py);
                PythonError::Call("reset")
            })?;
        self.re_acquire_gil();
        Ok(())
    }
}

pub type Request<T, U> = (T, oneshot::Sender<U>);
pub type RequestRx<T, U> = UnboundedReceiver<Request<T, U>>;
pub type RequestTx<T, U> = UnboundedSender<Request<T, U>>;

pub fn spawn_py_aggregator(
    settings: PythonAggregatorSettings,
) -> (PyAggregatorHandle, Receiver<()>) {
    let (aggregate_tx, aggregate_rx) =
        unbounded_channel::<Request<(), Result<Bytes, PyAggregatorError<AggregationFailed>>>>();

    let (add_weights_tx, add_weights_rx) =
        unbounded_channel::<Request<Bytes, Result<(), PyAggregatorError<InvalidWeights>>>>();

    let (mut shutdown_tx, shutdown_rx) = channel::<()>(1);

    thread::spawn(move || {
        block_on(async move {
            let _ = py_aggregator(settings, aggregate_rx, add_weights_rx)
                .await
                .map_err(|e| {
                    error!(error=%e, "py_aggregator terminated with an error");
                });
            if shutdown_tx.send(()).await.is_err() {
                warn!("py_aggregator: could not send shutdown signal (receiver is closed)");
            }
        });
    });

    let handle = PyAggregatorHandle {
        aggregate_requests: aggregate_tx,
        add_weights_requests: add_weights_tx,
    };
    (handle, shutdown_rx)
}

pub struct PyAggregatorHandle {
    pub aggregate_requests: RequestTx<(), Result<Bytes, PyAggregatorError<AggregationFailed>>>,
    pub add_weights_requests: RequestTx<Bytes, Result<(), PyAggregatorError<InvalidWeights>>>,
}

impl Aggregator for PyAggregatorHandle {
    type Error = PyAggregatorHandleError;
    type AggregateFut = Pin<Box<dyn Future<Output = Result<Bytes, Self::Error>> + Send>>;
    type AddWeightsFut = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send>>;

    fn add_weights(&mut self, weights: Bytes) -> Self::AddWeightsFut {
        let (tx, rx) = oneshot::channel::<Result<(), PyAggregatorError<InvalidWeights>>>();
        let add_weights_requests = self.add_weights_requests.clone();
        Box::pin(async move {
            add_weights_requests
                .send((weights, tx))
                .map_err(|_| ChannelError::Request)?;
            rx.await
                .map_err(|_| ChannelError::Response)?
                .map_err(From::from)
        })
    }

    fn aggregate(&mut self) -> Self::AggregateFut {
        let (tx, rx) = oneshot::channel::<Result<Bytes, PyAggregatorError<AggregationFailed>>>();
        let aggregate_requests = self.aggregate_requests.clone();
        Box::pin(async move {
            aggregate_requests
                .send(((), tx))
                .map_err(|_| ChannelError::Request)?;
            rx.await
                .map_err(|_| ChannelError::Response)?
                .map_err(From::from)
        })
    }
}

async fn py_aggregator(
    settings: PythonAggregatorSettings,
    mut aggregate_requests: RequestRx<(), Result<Bytes, PyAggregatorError<AggregationFailed>>>,
    mut add_weights_requests: RequestRx<Bytes, Result<(), PyAggregatorError<InvalidWeights>>>,
) -> Result<(), PythonError> {
    let mut aggregator = PyAggregator::load(settings)?;

    loop {
        select! {
            Some(((), resp_tx)) = aggregate_requests.recv() => {
                let weights = aggregator.aggregate();
                if resp_tx.send(weights).is_err() {
                    warn!("cannot send aggregate response: receiver is closed");
                    break;
                }

            }
            Some((weights, resp_tx)) = add_weights_requests.recv() => {
                let res = aggregator.add_weights(&weights[..]);
                if resp_tx.send(res).is_err() {
                    warn!("cannot send add_weights response: receiver is closed");
                    break;
                }
            }
            else => {
                warn!("PyAggregator shutting down: at least one receiver is closed");
                break;
            }
        }
    }

    // Clean shutdown of receivers: first close the channel to
    // prevent producers to push more values, then drain the
    // channels.
    aggregate_requests.close();
    while aggregate_requests.try_recv().is_ok() {}

    add_weights_requests.close();
    while add_weights_requests.try_recv().is_ok() {}

    Ok(())
}

#[derive(Error, Debug)]
#[error("the aggregation failed")]
pub struct AggregationFailed;

#[derive(Error, Debug)]
#[error("the weights are invalid and were rejected by the aggregator")]
pub struct InvalidWeights;

#[derive(Error, Debug)]
pub enum PyAggregatorError<E>
where
    E: Error,
{
    #[error("error while executing Python code: {0}")]
    Python(#[from] PythonError),
    #[error("request failed: {0}")]
    Request(E),
}

#[derive(Error, Debug)]
pub enum PyAggregatorHandleError {
    #[error("failed to send the request or receive the response")]
    Handle(#[from] ChannelError),

    #[error("error while executing Python code")]
    Python(#[from] PythonError),

    #[error("request failed: {0}")]
    Request(String),
}

impl<E> From<PyAggregatorError<E>> for PyAggregatorHandleError
where
    E: Error,
{
    fn from(e: PyAggregatorError<E>) -> Self {
        match e {
            PyAggregatorError::Python(inner) => Self::Python(inner),
            PyAggregatorError::Request(inner) => Self::Request(format!("{}", inner)),
        }
    }
}

#[derive(Error, Debug)]
pub enum ChannelError {
    #[error("failed to send request to PyAggregator")]
    Request,
    #[error("failed to receive response from PyAggregator")]
    Response,
}

#[derive(Error, Debug)]
pub enum PythonError {
    #[error("call to `{0}` resulted in an exception")]
    Call(&'static str),

    #[error("failed to convert Python `{0}` into Rust `{1}`")]
    Cast(&'static str, &'static str),

    #[error("an unknown error occured while calling Python code: {0}")]
    Unknown(&'static str),

    #[error("failed to load python module `{0}`")]
    LoadModule(String),

    #[error("failed to load python class `{0}.{1}`")]
    LoadClass(String, String),
}

#[cfg(test)]
mod tests {

    use super::*;

    use pyo3::{
        conversion::FromPyObject,
        types::{PyDict, PyList},
    };

    fn generate_serialized_weights(value: i64) -> Vec<u8> {
        let gil = pyo3::Python::acquire_gil();
        let py = gil.python();
        let locals = PyDict::new(py);
        let code = format!(
            r#"
import numpy as np
from io import BytesIO

nb_samples = 1
weights = np.repeat({}.0, 10)
writer = BytesIO()
writer.write(nb_samples.to_bytes(4, byteorder="big"))
np.save(writer, weights, allow_pickle=False)
res = writer.getvalue()
"#,
            value
        );
        py.run(&code, None, Some(locals)).unwrap();
        let res = locals.get_item("res").unwrap();
        let bytes: &PyBytes = res.downcast().unwrap();
        bytes.as_bytes().to_vec()
    }

    fn deserialize_weights(value: &[u8]) -> Vec<f64> {
        let gil = pyo3::Python::acquire_gil();
        let py = gil.python();

        let locals = PyDict::new(py);
        locals.set_item("value", PyBytes::new(py, value)).unwrap();

        let code = r#"
import numpy as np
from io import BytesIO

reader = BytesIO(value)
res = list(np.load(reader, allow_pickle=False))
"#;
        py.run(&code, None, Some(locals)).unwrap();
        let res = locals.get_item("res").unwrap();
        let res: &PyList = res.downcast().unwrap();
        res.into_iter()
            .map(|any| f64::extract(any).unwrap())
            .collect()
    }

    fn spawn_weighted_average_aggregator() -> PyAggregator {
        let settings = PythonAggregatorSettings {
            module: String::from("xain_aggregators.weighted_average"),
            class: String::from("Aggregator"),
        };
        PyAggregator::load(settings).unwrap()
    }

    /// Load a new PyAggregator with valid settings.
    #[test]
    fn test_py_aggregator_load() {
        let _ = spawn_weighted_average_aggregator();
    }

    /// Try to load a PyAggregator with a module that does not exist.
    /// The returned value should be an error.
    #[test]
    fn test_py_aggregator_load_module_not_found() {
        let settings = PythonAggregatorSettings {
            module: String::from("no_module"),
            class: String::from("Aggregator"),
        };
        let res = PyAggregator::load(settings);
        assert_eq!(
            "failed to load python module `no_module`".to_string(),
            res.err().unwrap().to_string()
        );
    }

    /// Try to load a PyAggregator with a class that does not exist
    /// within the module. The returned value should be an error.
    #[test]
    fn test_py_aggregator_load_class_not_found() {
        let settings = PythonAggregatorSettings {
            module: String::from("xain_aggregators.weighted_average"),
            class: String::from("no_class"),
        };
        let res = PyAggregator::load(settings);
        assert_eq!(
            "failed to load python class `xain_aggregators.weighted_average.no_class`".to_string(),
            res.err().unwrap().to_string()
        );
    }

    /// Load a new `PythonAggregator` and call the `add_weights`
    /// method of an aggregator with an valid weight array.
    #[test]
    fn test_py_aggregator_add_weights() {
        let aggregator = spawn_weighted_average_aggregator();
        let data = generate_serialized_weights(1);
        let _ = aggregator.add_weights(&data[..]).unwrap();
    }

    /// Load a new `PythonAggregator` and call the `add_weights`
    /// method with invalid data. The returned value should be an
    /// error.
    #[test]
    fn test_py_aggregator_add_weights_invalid_data() {
        let aggregator = spawn_weighted_average_aggregator();

        let weights = [1, 2, 3, 4];

        let res = aggregator.add_weights(&weights[..]);
        assert!(res.is_err());
        assert_eq!(
            "error while executing Python code: call to `add_weights` resulted in an exception"
                .to_string(),
            res.err().unwrap().to_string()
        );
    }

    /// Load a new `PythonAggregator` and call the `add_weights`
    /// method with valid weight data. Call the aggregate method of
    /// the aggregator and verify that the returned value and the
    /// weight data are equal.
    #[test]
    fn test_py_aggregator_aggregate() {
        let mut aggregator = spawn_weighted_average_aggregator();

        let weights_0 = generate_serialized_weights(0);
        let _ = aggregator.add_weights(&weights_0[..]);

        let weights_4 = generate_serialized_weights(4);
        let _ = aggregator.add_weights(&weights_4[..]);

        let raw = aggregator.aggregate().unwrap();
        let aggregated_weights = deserialize_weights(&raw[..]);
        assert_eq!(aggregated_weights, vec![2.0; 10]);
    }

    /// Load a new PythonAggregator and call the aggregate method of
    /// the aggregator without calling the add_weights method before.
    /// The return value should be 0.0.
    #[test]
    fn test_py_aggregator_aggregate_without_calling_add_weights() {
        let mut aggregator = spawn_weighted_average_aggregator();

        let raw = aggregator.aggregate().unwrap();
        let aggregated_weights = deserialize_weights(&raw[..]);
        assert_eq!(aggregated_weights, vec![0.0]);
    }

    /// Load a new PythonAggregator and call the add_weights method
    /// with invalid weight data.  Call the aggregate method of the
    /// aggregator and verify that the aggregate method returns the
    /// value 0.0.
    #[test]
    fn test_py_aggregator_aggregate_with_error_on_add_weights() {
        let mut aggregator = spawn_weighted_average_aggregator();

        let res = aggregator.add_weights(&[1, 2, 3, 4][..]);
        assert!(res.is_err());

        let raw = aggregator.aggregate().unwrap();
        let aggregated_weights = deserialize_weights(&raw[..]);
        assert_eq!(aggregated_weights, vec![0.0]);
    }

    /// Load a new PythonAggregator and call the add_weights method
    /// with valid weight data. Call the aggregate method of the
    /// aggregator and verify that the returned value and the weight
    /// data are equal. Call the get_global_weights method of the
    /// aggregator and verify that the returned value and the weight
    /// data are equal.
    #[test]
    fn test_py_aggregator_get_global_weights() {
        let mut aggregator = spawn_weighted_average_aggregator();

        let data = generate_serialized_weights(1);
        let _ = aggregator.add_weights(&data[..]).unwrap();
        let _ = aggregator.aggregate().unwrap();

        let raw = aggregator.get_global_weights().unwrap();
        let aggregated_weights = deserialize_weights(&raw[..]);
        assert_eq!(aggregated_weights, vec![1.0; 10]);
    }

    /// Load a new PythonAggregator and call the get_global_weights
    /// method of the aggregator without calling the add_weights or
    /// the aggregate method before. The return value should be
    /// empty.
    #[test]
    fn test_py_aggregator_get_global_weights_without_calling_add_weights() {
        let aggregator = spawn_weighted_average_aggregator();
        let res = aggregator.get_global_weights().unwrap();
        assert_eq!(res.len(), 0);
    }

    /// Load a new PythonAggregator, set the global weights, and
    /// reset the global weights data. Verify that get_global_weights
    /// returns the same data as we passed to the reset method.
    #[test]
    fn test_py_aggregator_reset() {
        let mut aggregator = spawn_weighted_average_aggregator();

        let _ = aggregator
            .add_weights(&generate_serialized_weights(1)[..])
            .unwrap();
        let _ = aggregator.aggregate();

        let _ = aggregator
            // Ignore the first four bytes, because unlike
            // `add_weights`, `reset` doesn't expect a number of
            // samples!
            .reset(&generate_serialized_weights(2)[4..])
            .unwrap();

        let raw = aggregator.get_global_weights().unwrap();
        let aggregated_weights = deserialize_weights(&raw[..]);
        assert_eq!(aggregated_weights, vec![2.0; 10]);
    }

    /// Load a new PythonAggregator and call the reset method with
    /// invalid data. The returned value should be an error.
    #[test]
    fn test_py_aggregator_reset_invalid_global_weights() {
        let mut aggregator = spawn_weighted_average_aggregator();
        let res = aggregator.reset(&[1, 2, 3, 4][..]);
        assert_eq!(
            "call to `reset` resulted in an exception".to_string(),
            res.err().unwrap().to_string()
        );
    }
}
