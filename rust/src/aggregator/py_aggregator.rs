use anyhow::{Context, Result};
use bytes::Bytes;
use futures::{executor::block_on, TryFutureExt};
use std::{future::Future, pin::Pin, thread};
use thiserror::Error;
use tokio::{
    select,
    sync::{
        mpsc::{
            unbounded_channel as channel, UnboundedReceiver as Receiver, UnboundedSender as Sender,
        },
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
pub type RequestRx<T, U> = Receiver<Request<T, U>>;
pub type RequestTx<T, U> = Sender<Request<T, U>>;

pub fn spawn_py_aggregator(settings: PythonAggregatorSettings) -> PyAggregatorHandle {
    let (aggregate_tx, aggregate_rx) = channel::<Request<(), Weights>>();
    let (add_weights_tx, add_weights_rx) = channel::<Request<Weights, ()>>();
    thread::spawn(move || {
        block_on(
            py_aggregator(settings, aggregate_rx, add_weights_rx)
                .map_err(|e| error!("py_aggregator failure: {}", e)),
        )
    });
    PyAggregatorHandle {
        aggregate_requests: aggregate_tx,
        add_weights_requests: add_weights_tx,
    }
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
