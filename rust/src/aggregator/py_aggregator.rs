use anyhow::{Context, Result};
use bytes::Bytes;
use futures::executor::block_on;
use std::{future::Future, pin::Pin, thread};
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
    pub fn load(settings: PythonAggregatorSettings) -> Result<Self> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let module = PyModule::import(py, &settings.module).map_err(|e| {
            // Currently, there is no easy way to convert `PyErr` into
            // a Rust error type so we just print the error on
            // stderr. See: https://github.com/PyO3/pyo3/issues/592
            // and https://github.com/PyO3/pyo3/issues/682
            e.print(py);
            PyAggregatorError::LoadModule(settings.module.clone())
        })?;
        let aggregator = module
            .call0(&settings.class)
            .map_err(|e| {
                // Currently, there is no easy way to convert `PyErr` into
                // a Rust error type so we just print the error on
                // stderr. See: https://github.com/PyO3/pyo3/issues/592
                // and https://github.com/PyO3/pyo3/issues/682
                e.print(py);
                PyAggregatorError::LoadClass(settings.module.clone(), settings.class.clone())
            })?
            .to_object(py);
        Ok(Self {
            gil: Some(gil),
            aggregator,
        })
    }

    pub fn aggregate(&mut self) -> Result<Bytes> {
        info!("PyAggregator: running aggregation");
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
                PyAggregatorError::Call("aggregate")
            })?
            .extract::<Vec<u8>>(py)
            .map(Bytes::from)
            .map_err(|e| {
                // Currently, there is no easy way to convert `PyErr` into
                // a Rust error type so we just print the error on
                // stderr. See: https://github.com/PyO3/pyo3/issues/592
                // and https://github.com/PyO3/pyo3/issues/682
                e.print(py);
                PyAggregatorError::Unknown("Failed to convert Python `bytes` into Rust `Vec<u8>`")
            })?;
        info!("PyAggregator: finished aggregation");
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

    pub fn add_weights(&self, local_weights: &[u8]) -> Result<::std::result::Result<(), ()>> {
        info!("PyAggregator: adding weights");
        let py = self.get_py();
        let py_bytes = PyBytes::new(py, local_weights);
        let args = (py_bytes,);
        let result = self
            .aggregator
            .call_method1(py, "add_weights", args)
            .map_err(|e| {
                // Currently, there is no easy way to convert `PyErr` into
                // a Rust error type so we just print the error on
                // stderr. See: https://github.com/PyO3/pyo3/issues/592
                // and https://github.com/PyO3/pyo3/issues/682
                e.print(py);
                PyAggregatorError::Call("add_weights")
            })?
            .extract::<bool>(py)
            .map_err(|e| {
                // Currently, there is no easy way to convert `PyErr` into
                // a Rust error type so we just print the error on
                // stderr. See: https://github.com/PyO3/pyo3/issues/592
                // and https://github.com/PyO3/pyo3/issues/682
                e.print(py);
                PyAggregatorError::Unknown("Failed to convert Python `bool` into Rust `bool`")
            })?
            .then_some(())
            .ok_or(());
        info!("PyAggregator: done adding weights");
        Ok(result)
    }

    pub fn get_py(&self) -> Python<'_> {
        // UNWRAP_SAFE: As long as PyAggregator exists, self.gil
        // cannot be None: the only place where we temporarily set it
        // to None is in PyAggregator.re_acquire_gil(), but we set it
        // back to Some right away.
        self.gil.as_ref().unwrap().python()
    }

    pub fn reset(&mut self, global_weights: &[u8]) -> Result<()> {
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
                PyAggregatorError::Call("reset")
            })?;
        self.re_acquire_gil();
        Ok(())
    }
}

pub type Weights = Bytes;
pub type Request<T, U> = (T, oneshot::Sender<U>);
pub type RequestRx<T, U> = UnboundedReceiver<Request<T, U>>;
pub type RequestTx<T, U> = UnboundedSender<Request<T, U>>;

pub fn spawn_py_aggregator(
    settings: PythonAggregatorSettings,
) -> (PyAggregatorHandle, Receiver<()>) {
    let (aggregate_tx, aggregate_rx) = unbounded_channel::<Request<(), Weights>>();
    let (add_weights_tx, add_weights_rx) = unbounded_channel::<Request<Weights, ()>>();
    let (mut shutdown_tx, shutdown_rx) = channel::<()>(1);

    thread::spawn(move || {
        block_on(async move {
            if let Err(e) = py_aggregator(settings, aggregate_rx, add_weights_rx).await {
                error!("py_aggregator failure: {}", e);
            }
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
    pub aggregate_requests: RequestTx<(), Weights>,
    pub add_weights_requests: RequestTx<Weights, ()>,
}

impl Aggregator for PyAggregatorHandle {
    type Error = ();
    type AggregateFut = Pin<Box<dyn Future<Output = Result<Bytes, ()>>>>;
    type AddWeightsFut = Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>;

    fn add_weights(&mut self, weights: Bytes) -> Self::AddWeightsFut {
        let (tx, rx) = oneshot::channel::<()>();
        let add_weights_requests = self.add_weights_requests.clone();
        Box::pin(async move {
            add_weights_requests.send((weights, tx)).map_err(|_| ())?;
            rx.await.map_err(|_| ())
        })
    }

    fn aggregate(&mut self) -> Self::AggregateFut {
        let (tx, rx) = oneshot::channel::<Bytes>();
        let aggregate_requests = self.aggregate_requests.clone();
        Box::pin(async move {
            aggregate_requests.send(((), tx)).map_err(|_| ())?;
            rx.await.map_err(|_| ())
        })
    }
}

async fn py_aggregator(
    settings: PythonAggregatorSettings,
    mut aggregate_requests: RequestRx<(), Weights>,
    mut add_weights_requests: RequestRx<Weights, ()>,
) -> Result<()> {
    let mut aggregator = PyAggregator::load(settings)?;

    loop {
        select! {
            Some(((), resp_tx)) = aggregate_requests.recv() => {
                let weights = aggregator.aggregate().context("aggregation failed")?;
                if resp_tx.send(weights).is_err() {
                    warn!("cannot send aggregate response: receiver is closed");
                    break;
                }

            }
            Some((weights, resp_tx)) = add_weights_requests.recv() => {
                // FIXME: don't unwrap here. We need to send the
                // result.
                aggregator.add_weights(&weights[..]).context("failed to add weights")?.unwrap();
                if resp_tx.send(()).is_err() {
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
pub enum PyAggregatorError {
    #[error("failed to load python module `{0}`")]
    LoadModule(String),
    #[error("failed to load python class `{0}.{1}`")]
    LoadClass(String, String),
    #[error("call to `Aggregator.{0}()` resulted in an exception")]
    Call(&'static str),
    #[error("an unknown error occured while calling Python code: {0}")]
    Unknown(&'static str),
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_py_aggregator_load() {
        // Load a new PyAggregator with valid settings.
        let settings = PythonAggregatorSettings {
            module: String::from("xain_aggregators.weighted_average"),
            class: String::from("Aggregator"),
        };

        let res = PyAggregator::load(settings);

        assert!(res.is_ok());
    }

    #[test]
    fn test_py_aggregator_load_module_not_found() {
        // Try to load a PyAggregator with a module that does not exist.
        // The returned value should be an error.
        let settings = PythonAggregatorSettings {
            module: String::from("no_module"),
            class: String::from("Aggregator"),
        };

        let res = PyAggregator::load(settings);

        assert!(res.is_err());
        assert_eq!(
            "failed to load python module `no_module`".to_string(),
            res.err().unwrap().to_string()
        );
    }

    #[test]
    fn test_py_aggregator_load_class_not_found() {
        // Try to load a PyAggregator with a class that does not exist within the module.
        // The returned value should be an error.
        let settings = PythonAggregatorSettings {
            module: String::from("xain_aggregators.weighted_average"),
            class: String::from("no_class"),
        };

        let res = PyAggregator::load(settings);

        assert!(res.is_err());
        assert_eq!(
            "failed to load python class `xain_aggregators.weighted_average.no_class`".to_string(),
            res.err().unwrap().to_string()
        );
    }
    #[test]
    fn test_py_aggregator_add_weights() {
        // Call the add_weights method of an aggregator with a valid weight array.
        let settings = PythonAggregatorSettings {
            module: String::from("xain_aggregators.weighted_average"),
            class: String::from("Aggregator"),
        };

        let aggregator = PyAggregator::load(settings).unwrap();

        // How to create a weights array via Python:
        //
        // import pickle
        // import numpy as np
        // weights = np.array([1] * 10)
        // training_result_data = int(0).to_bytes(4, byteorder="big") + pickle.dumps(weights)
        // print(training_result_data)
        let weights = b"\x00\x00\x00\x00\x80\x03cnumpy.core.multiarray\n_reconstruct\nq\x00cnumpy\nndarray\nq\x01K\x00\x85q\x02C\x01bq\x03\x87q\x04Rq\x05(K\x01K\n\x85q\x06cnumpy\ndtype\nq\x07X\x02\x00\x00\x00i8q\x08K\x00K\x01\x87q\tRq\n(K\x03X\x01\x00\x00\x00<q\x0bNNNJ\xff\xff\xff\xffJ\xff\xff\xff\xffK\x00tq\x0cb\x89CP\x01\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00q\rtq\x0eb.";

        let res = aggregator.add_weights(&weights[..]);
        assert!(res.is_ok());
        assert_eq!(res.ok().unwrap().ok(), Some(()));
    }
}
