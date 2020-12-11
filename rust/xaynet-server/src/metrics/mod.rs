pub mod recorders;
pub use self::recorders::influxdb::{Measurement, Recorder, Tags};
use once_cell::sync::OnceCell;

static RECORDER: OnceCell<Recorder> = OnceCell::new();

pub struct GlobalRecorder;

impl GlobalRecorder {
    /// Gets the reference to the global recorder.
    /// Returns `None` if no recorder is set or is currently being initialized.
    /// This method never blocks.
    pub fn global() -> Option<&'static Recorder> {
        RECORDER.get()
    }

    /// Installs a new global recorder.
    /// Returns Err(Recorder) if a recorder has already been set.
    pub fn install(recorder: Recorder) -> Result<(), Recorder> {
        RECORDER.set(recorder)
    }
}

/// Records an event.
///
/// # Example
///
/// ```rust
/// // An event with just a title:
/// event!("Error");
///
/// // An event with a title and a description:
/// event!("Error", "something went wrong");
///
/// // An event with a title, a description and tags:
/// event!(
///     "Error",
///     "something went wrong",
///     ["phase error", "coordinator"]
/// );
/// ```
#[macro_export]
macro_rules! event {
    ($title: expr) => {
        if let Some(recorder) = crate::metrics::GlobalRecorder::global() {
            recorder.event($title, None, None)
        }
    };
    ($title: expr, $description: expr) => {
        if let Some(recorder) = crate::metrics::GlobalRecorder::global() {
            recorder.event($title, Some($description), None)
        }
    };
    ($title: expr, $description: expr, $tags: expr) => {
        if let Some(recorder) = crate::metrics::GlobalRecorder::global() {
            recorder.event($title, Some($description), Some(&$tags))
        }
    };
}

/// Records a metric.
///
/// # Example
///
/// ```rust
/// // A basic metric:
/// metric!(Measurement::RoundTotalNumber, 1);
///
/// // A metric with one tag:
/// metric!(Measurement::RoundParamSum, 0.7, ("round_id", 1));
///
/// // A metric with multiple tags:
/// metric!(
///     Measurement::RoundParamSum,
///     0.7,
///     ("round_id", 1),
///     ("phase", 2)
/// );
/// ```
#[macro_export]
macro_rules! metric {
    ($measurement: expr, $value: expr) => {
        if let Some(recorder) = crate::metrics::GlobalRecorder::global() {
            recorder.metric($measurement, $value, None)
        }
    };
    ($measurement: expr, $value: expr, $($tag: expr),*) => {
        if let Some(recorder) = crate::metrics::GlobalRecorder::global() {
            let mut tags = crate::metrics::Tags::new();

            $(
                tags.add($tag.0, $tag.1);
            )*

            recorder.metric($measurement, $value, Some(tags))
        }
    };
}
