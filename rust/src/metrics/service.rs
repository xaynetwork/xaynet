use influxdb::{Client, WriteQuery};
use tokio::sync::mpsc::{channel, Receiver, Sender};

/// Runs the metrics service.
///
/// The future is automatically resolved after all senders have been dropped and all remaining
/// messages have been processed.
///
/// If an error occurs when sending the metric to InfluxDB, the metric is discarded and the error
/// is logged.
pub async fn run_metric_service(mut metrics_service: MetricsService) {
    loop {
        match metrics_service.receiver.recv().await {
            Some(write_query) => {
                let _ = metrics_service
                    .client
                    .query(&write_query)
                    .await
                    .map_err(|e| error!("{}", e));
            }
            None => {
                warn!("All senders have been dropped!");
                return;
            }
        }
    }
}

/// A handle to send metrics to the [`MetricsService`] via a bounded channel.
#[derive(Debug, Clone)]
pub struct MetricsSender(Sender<WriteQuery>);

impl MetricsSender {
    /// Sends a metric to the [`MetricsService`].
    /// If the channel is already full or closed, the metric is discarded and the error is logged.
    pub fn send(&mut self, query: WriteQuery) {
        let _ = self.0.try_send(query).map_err(|e| error!("{}", e));
    }
}

/// A service that handles the transmission of metrics to InfluxDB.
pub struct MetricsService {
    /// The InfluxDB client.
    client: Client,
    /// The receiver half of the bounded channel.
    receiver: Receiver<WriteQuery>,
}

impl MetricsService {
    /// Creates and returns a new [`MetricsService`] and the associated [`MetricsSender`].
    /// The [`MetricsSender`] can be used to send metrics to the [`MetricsService`].
    /// The [`MetricsService`] handles the transmission of metrics to InfluxDB.
    ///
    /// - `url`: The url where InfluxDB is running (e.g. `http://127.0.0.1:8086`).
    /// - `database`: The name of the database in which the metrics are to be written.
    ///
    /// Note:
    /// It is assumed that the database already exists. If this is not the case, no metrics are
    /// written in InfluxDB.
    pub fn new(url: &str, database: &str) -> (MetricsService, MetricsSender) {
        let client = Client::new(url, database);
        Self::new_metrics_service(client)
    }

    /// Similar to the [`MetricsService::new`] but with additional InfluxDB user credentials.
    ///
    /// - `username`: The username for InfluxDB.
    /// - `password`: The password for that username.
    pub fn new_with_auth(
        url: &str,
        database: &str,
        username: &str,
        password: &str,
    ) -> (MetricsService, MetricsSender) {
        let client_auth = Client::new(url, database).with_auth(username, password);
        Self::new_metrics_service(client_auth)
    }

    fn new_metrics_service(client: Client) -> (MetricsService, MetricsSender) {
        let (sender, receiver) = channel(4096);
        (MetricsService { client, receiver }, MetricsSender(sender))
    }
}
