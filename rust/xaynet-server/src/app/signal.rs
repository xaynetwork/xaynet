/// copied from https://github.com/linkerd/linkerd2-proxy/blob/f12988b773796b4fe46f5554bf5c2ad4b28f7f9b/linkerd/signal/src/lib.rs

/// Returns a `Future` that completes when the proxy should start to shutdown.
pub async fn shutdown() {
    imp::shutdown().await
}

#[cfg(unix)]
mod imp {
    use tokio::signal::unix::{signal, SignalKind};
    use tracing::info;

    pub(super) async fn shutdown() {
        tokio::select! {
            // SIGINT  - To allow Ctrl-c to emulate SIGTERM while developing.
            () = sig(SignalKind::interrupt(), "SIGINT") => {}
            // SIGTERM - Kubernetes sends this to start a graceful shutdown.
            () = sig(SignalKind::terminate(), "SIGTERM") => {}
        };
    }

    async fn sig(kind: SignalKind, name: &'static str) {
        // Create a Future that completes the first
        // time the process receives 'sig'.
        signal(kind)
            .expect("Failed to register signal handler")
            .recv()
            .await;
        info!(
            // use target to remove 'imp' from output
            target: "xaynet_server::signal",
            "received {}, starting shutdown",
            name,
        );
    }
}

#[cfg(not(unix))]
mod imp {
    use futures::prelude::*;
    use tracing::info;

    pub(super) async fn shutdown() {
        // On Windows, we don't have all the signals, but Windows also
        // isn't our expected deployment target. This implementation allows
        // developers on Windows to simulate proxy graceful shutdown
        // by pressing Ctrl-C.
        tokio::signal::ctrl_c().recv().await;
        info!(
            // use target to remove 'imp' from output
            target: "xaynet_server::signal",
            "received Ctrl-C, starting shutdown",
        );
    }
}
