//! Utils to record metrics.

pub mod recorders;

use once_cell::sync::OnceCell;

pub use self::recorders::influxdb::{Measurement, Recorder, Tags};

static RECORDER: OnceCell<Recorder> = OnceCell::new();

/// A wrapper around a static global metrics/events recorder.
pub struct GlobalRecorder;

impl GlobalRecorder {
    /// Gets the reference to the global recorder.
    ///
    /// Returns `None` if no recorder is set or is currently being initialized.
    /// This method never blocks.
    pub fn global() -> Option<&'static Recorder> {
        RECORDER.get()
    }

    /// Installs a new global recorder.
    ///
    /// Returns Err(Recorder) if a recorder has already been set.
    pub fn install(recorder: Recorder) -> Result<(), Recorder> {
        RECORDER.set(recorder)
    }
}

/// Records an event.
///
/// # Example
///
/// ```ignore
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
///     ["phase error", "coordinator"],
/// );
/// ```
#[macro_export]
macro_rules! event {
    ($title: expr $(,)?) => {
        if let Some(recorder) = crate::metrics::GlobalRecorder::global() {
            recorder.event::<_, _, &str, _, &[_], &str>($title, None, None);
        }
    };
    ($title: expr, $description: expr $(,)?) => {
        if let Some(recorder) = crate::metrics::GlobalRecorder::global() {
            recorder.event::<_, _, _, _, &[_], &str>($title, $description, None);
        }
    };
    ($title: expr, $description: expr, [$($tags: expr),+] $(,)?) => {
        if let Some(recorder) = crate::metrics::GlobalRecorder::global() {
            recorder.event($title, $description, [$($tags),+])
        }
    };
}

/// Records a metric.
///
/// # Example
///
/// ```ignore
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
///     ("phase", 2),
/// );
/// ```
#[macro_export]
macro_rules! metric {
    (accepted: $round_id: expr, $phase: expr $(,)?) => {
        crate::metric!(
            crate::metrics::Measurement::MessageAccepted,
            1,
            ("round_id", $round_id),
            ("phase", $phase as u8),
        );
    };
    (rejected: $round_id: expr, $phase: expr $(,)?) => {
        crate::metric!(
            crate::metrics::Measurement::MessageRejected,
            1,
            ("round_id", $round_id),
            ("phase", $phase as u8),
        );
    };
    (discarded: $round_id: expr, $phase: expr $(,)?) => {
        crate::metric!(
            crate::metrics::Measurement::MessageDiscarded,
            1,
            ("round_id", $round_id),
            ("phase", $phase as u8),
        );
    };
    ($measurement: expr, $value: expr $(,)?) => {
        if let Some(recorder) = crate::metrics::GlobalRecorder::global() {
            recorder.metric::<_, _, crate::metrics::Tags>($measurement, $value, None);
        }
    };
    ($measurement: expr, $value: expr, $(($tag: expr, $val: expr)),+ $(,)?) => {
        if let Some(recorder) = crate::metrics::GlobalRecorder::global() {
            let mut tags = crate::metrics::Tags::new();
            $(
                tags.add($tag, $val);
            )+
            recorder.metric($measurement, $value, tags);
        }
    };
}
