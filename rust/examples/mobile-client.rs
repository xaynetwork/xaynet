/// Experimental mobile client
extern crate tracing;

use std::io::{stdin, stdout, Read, Write};
use structopt::StructOpt;
use tracing_subscriber::*;
use xaynet_client::mobile_client::{
    participant::{AggregationConfig, ParticipantSettings},
    MobileClient,
};
use xaynet_core::mask::{
    BoundType,
    DataType,
    FromPrimitives,
    GroupType,
    IntoPrimitives,
    MaskConfig,
    Model,
    ModelType,
};

#[derive(Debug, StructOpt)]
#[structopt(name = "Test Drive")]
struct Opt {
    #[structopt(
        default_value = "http://127.0.0.1:8081",
        short,
        help = "The URL of the coordinator"
    )]
    url: String,
    #[structopt(default_value = "4", short, help = "The length of the model")]
    len: u32,
}

fn pause() {
    let mut stdout = stdout();
    stdout.write_all(b"Press Enter to continue...").unwrap();
    stdout.flush().unwrap();
    stdin().read_exact(&mut [0]).unwrap();
}

fn get_participant_settings() -> ParticipantSettings {
    sodiumoxide::init().unwrap();

    let secret_key = MobileClient::create_participant_secret_key();
    ParticipantSettings {
        secret_key,
        aggregation_config: AggregationConfig {
            mask: MaskConfig {
                group_type: GroupType::Prime,
                data_type: DataType::F32,
                bound_type: BoundType::B0,
                model_type: ModelType::M3,
            },
            scalar: 1_f64,
        },
    }
}

// // How a Dart API could look like:

// // only needs to be executed once (first start of the app)
// init_client() {
//     // first check if the participant exist in the database
//     ...
//
//     // generates a fresh client secret key
//     var secret_key = createSecretKeys();

//     // init a new client
//     var client = MobileClient::init(coordinator_url, secret_key, other_participant_settings);

//     // serialize the client state (includes the secret key)
//     var serialized_client = client.serialize();

//     // save the state in the database
//     db.save("client_state", serialized_client);
// }

// perform_task() {
//     // load the state from the database
//     var serialized_client = db.load("client_state");

//     // deserialize the client state
//     var client = MobileClient::deserialize(coordinator_url, serialized_client);

//     // load the latest local model from the database
//     var local_model = db.load("local_model");

//     // set the current local model
//     client.set_local_model(local_model);

//     // perform the participant task (this will change the internal state)
//     client.perform_task();

//     // get the global model
//     var global_model = client.global_model();

//     // save the global model
//     db.save("global_model", global_model);

//     // serialize the new state
//     serialized_client = client.serialize();

//     // override the old state with the new one
//     db.save("client_state", serialized_client);
// }
fn main() -> Result<(), ()> {
    let opt = Opt::from_args();

    let _fmt_subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .init();

    // create a new client
    let client = MobileClient::init(&opt.url, get_participant_settings()).unwrap();
    // serialize the current client state (and save it on the phone)
    let mut bytes = client.serialize();

    // simulate the regular execution of perform_task on the phone
    loop {
        // load local model
        let model = Model::from_primitives(vec![1; opt.len as usize].into_iter()).unwrap();
        bytes = perform_task(&opt.url, &bytes, model);
        pause();
    }
}

// perform the participant task (this function should be triggered regularly on the phone while the
// app is active or in a background task)
fn perform_task(url: &str, bytes: &[u8], model: Model) -> Vec<u8> {
    let mut client = MobileClient::restore(url, bytes).unwrap();
    println!("task: {:?}", &client.get_current_state());

    client.set_local_model(model);
    client = match client.try_to_proceed() {
        Ok(client) => client,
        Err((client, _)) => client,
    };

    match client.get_global_model().unwrap() {
        Some(model) => println!(
            "global model: {:?}",
            model.into_primitives_unchecked().collect::<Vec<f32>>()
        ),
        _ => (),
    };

    let new_bytes = client.serialize();
    println!("size serialized: {:?}", &bytes.len());
    new_bytes
}
