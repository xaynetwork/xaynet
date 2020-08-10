use std::{path::PathBuf, process};
use structopt::StructOpt;
use tracing_subscriber::*;
use xaynet::{rest, services, settings::Settings, state_machine::StateMachine};

#[derive(Debug, StructOpt)]
#[structopt(name = "Coordinator")]
struct Opt {
    /// Path of the configuration file
    #[structopt(short, parse(from_os_str))]
    config_path: PathBuf,
}
#[tokio::main]
async fn main() {
    let opt = Opt::from_args();
    let Settings {
        pet: pet_settings,
        mask: mask_settings,
        api: api_settings,
        log: log_settings,
        model: model_settings,
        metrics: _metrics_settings,
    } = Settings::new(opt.config_path).unwrap_or_else(|err| {
        eprintln!("{}", err);
        process::exit(1);
    });

    let _fmt_subscriber = FmtSubscriber::builder()
        .with_env_filter(log_settings.filter)
        .with_ansi(true)
        .init();

    // This should already called internally when instantiating the
    // state machine but it doesn't hurt making sure the crypto layer
    // is correctly initialized
    sodiumoxide::init().unwrap();

    let (state_machine, requests_tx, event_subscriber) =
        StateMachine::new(pet_settings, mask_settings, model_settings).unwrap();
    let fetcher = services::fetcher(&event_subscriber);
    let message_handler = services::message_handler(&event_subscriber, requests_tx);

    tokio::select! {
        _ = state_machine.run() => {
            println!("shutting down: Service terminated");
        }
        _ = rest::serve(api_settings.bind_address, fetcher, message_handler) => {
            println!("shutting down: REST server terminated");
        }
    }
}
