use bytes::Bytes;
use futures::executor::block_on;
use std::{future::Future, pin::Pin, thread};
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
    pub fn load(settings: PythonAggregatorSettings) -> PyResult<Self> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        // FIXME: make this configurable
        let module = PyModule::import(py, &settings.module)
            .map_err(|e| e.print(py))
            .unwrap();
        let aggregator = module.call0(&settings.class).unwrap().to_object(py);
        Ok(Self {
            gil: Some(gil),
            aggregator,
        })
    }

    pub fn aggregate(&mut self) -> PyResult<Bytes> {
        info!("PyAggregator: running aggregation");
        let py = self.get_py();
        let result = self
            .aggregator
            .call_method0(py, "aggregate")?
            .extract::<Vec<u8>>(py)
            .map(Bytes::from)?;
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

    pub fn add_weights(&self, local_weights: &[u8]) -> PyResult<Result<(), ()>> {
        info!("PyAggregator: adding weights");
        let py = self.get_py();
        let py_bytes = PyBytes::new(py, local_weights);
        let args = (py_bytes,);
        let result = self
            .aggregator
            .call_method1(py, "add_weights", args)?
            .extract::<bool>(py)?
            .then_some(())
            .ok_or(());
        info!("PyAggregator: done adding weights");
        Ok(result)
    }

    pub fn get_py(&self) -> Python<'_> {
        self.gil.as_ref().unwrap().python()
    }

    pub fn reset(&mut self, global_weights: &[u8]) -> PyResult<()> {
        let py = self.get_py();
        let py_bytes = PyBytes::new(py, global_weights);
        let args = (py_bytes,);
        self.aggregator.call_method1(py, "reset", args)?;
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
    thread::spawn(move || block_on(py_aggregator(settings, aggregate_rx, add_weights_rx)));
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

// FIXME: remove the unwraps
async fn py_aggregator(
    settings: PythonAggregatorSettings,
    mut aggregate_requests: RequestRx<(), Weights>,
    mut add_weights_requests: RequestRx<Weights, ()>,
) {
    let mut aggregator = PyAggregator::load(settings).unwrap();

    loop {
        select! {
            Some(((), resp_tx)) = aggregate_requests.recv() => {
                let weights = aggregator.aggregate().map_err(|e| error!("{:?}", e)).unwrap();
                if resp_tx.send(weights).is_err() {
                    warn!("cannot send aggregate response, receiver has been dropped");
                    return;
                }

            }
            Some((weights, resp_tx)) = add_weights_requests.recv() => {
                aggregator.add_weights(&weights[..]).unwrap().unwrap();
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
