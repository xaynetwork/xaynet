use clap::Arg;

type DynError = Box<dyn std::error::Error>;

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{}", e);
        std::process::exit(-1);
    }
}

fn try_main() -> Result<(), DynError> {
    let cli_opts = CliOpts::parse()?;
    println!("{:?}", cli_opts);
    Ok(())
}

#[derive(Debug)]
struct CliOpts {
    n_clients: u32,
    n_rounds: u32,
    dataset: String,
}

impl CliOpts {
    fn parse() -> Result<CliOpts, DynError> {
        let matches = clap::App::new("xain-coordinator")
            .version("0.1")
            .arg(Arg::with_name("dataset").takes_value(true).required(true))
            .arg(Arg::with_name("clients").long("clients").takes_value(true).required(true))
            .arg(Arg::with_name("rounds").long("rounds").takes_value(true).required(true))
            .get_matches();

        let res = CliOpts {
            n_clients: value_of_u32(&matches, "clients")?,
            n_rounds: value_of_u32(&matches, "rounds")?,
            dataset: matches.value_of("dataset").unwrap().to_string(),
        };
        return Ok(res);

        fn value_of_u32(matches: &clap::ArgMatches, name: &str) -> Result<u32, DynError> {
            let arg = matches.value_of(name).unwrap();
            arg.parse::<u32>().map_err(|_| format!("could not parse `{}` as a number", arg).into())
        }
    }
}
