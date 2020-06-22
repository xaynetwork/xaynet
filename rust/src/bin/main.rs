use std::{path::PathBuf, process};
use structopt::StructOpt;
use tracing_subscriber::*;
use xain_fl::{rest, service::Service, settings::Settings};

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
    let settings = Settings::new(opt.config_path).unwrap_or_else(|err| {
        eprintln!("{}", err);
        process::exit(1);
    });

    let _fmt_subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .init();

    let (service, handle) = Service::new(settings.pet, settings.mask).unwrap();

    tokio::select! {
        _ = service => {
            println!("shutting down: Service terminated");
        }
        _ = rest::serve(settings.api.bind_address, handle.clone()) => {
            println!("shutting down: REST server terminated");
        }
    }
}
