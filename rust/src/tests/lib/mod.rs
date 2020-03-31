pub mod aggregator;
pub mod coordinator;
pub mod rpc;

use crate::common::{logging, settings::LoggingSettings};
use tokio::time::{delay_for, Duration};
use tracing_subscriber::filter::EnvFilter;

/// This function makes it easy to toggle logging in the test. If
/// called, and if the `RUST_LOG` environment variable is set, the value is used as a filter for tracing. For instance, to have the logs dumped during the tests one can do:
///
/// ```no_rust
/// TEST_LOGS=trace cargo test
/// ```
pub fn enable_logging() {
    ::std::env::var("TEST_LOGS").ok().map(|filter| {
        logging::configure(LoggingSettings {
            telemetry: None,
            filter: EnvFilter::try_new(filter).unwrap(),
        });
    });
}

/// Sleep for the given amount of time, in milliseconds. Note that in
/// tokio tests, we MUST NOT call `::std::thread::sleep` because it
/// blocks the event loop.
pub async fn sleep_ms(ms: u64) {
    delay_for(Duration::from_millis(ms)).await
}
