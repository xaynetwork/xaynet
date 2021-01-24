use tracing_subscriber::{fmt::Formatter, reload::Handle, EnvFilter, FmtSubscriber};

#[derive(Clone)]
pub struct Tracing {
    filter_handle: Handle<EnvFilter, Formatter>,
}

impl Tracing {
    pub fn new() -> Self {
        let fmt_subscriber = FmtSubscriber::builder()
            .with_env_filter("debug")
            .with_ansi(true)
            .with_filter_reloading();
        let filter_handle = fmt_subscriber.reload_handle();
        fmt_subscriber.init();

        Self { filter_handle }
    }

    pub fn reload(&self, filter: impl Into<EnvFilter>) -> Result<(), ()> {
        self.filter_handle.reload(filter).map_err(|_| ())
    }

    pub fn set_from(&self, bytes: bytes::Bytes) -> Result<(), ()> {
        tracing::info!("{:?}", bytes);
        let body = std::str::from_utf8(&bytes.as_ref()).map_err(|_| ())?;
        tracing::info!("body {:?}", body);
        self.reload(body)
    }
}

impl Default for Tracing {
    fn default() -> Self {
        Tracing::new()
    }
}
