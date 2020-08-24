fn main() {}
// use tracing_subscriber::*;
// use xaynet::{
//     client::{participant::Task, Client, ClientError},
//     mask::{FromPrimitives, Model},
//     service::Service,
//     settings::{MaskSettings, PetSettings},
// };

// /// Test-drive script of a (local) single-round federated learning session,
// /// intended for use as a mini integration test. It spawns a [`Service`] and 10
// /// [`Client`]s on the tokio event loop. This serves as a simple example of
// /// getting started with the project, and may later be the basis for more
// /// automated tests.
// #[tokio::main]
// async fn main() -> Result<(), ClientError> {
//     let _fmt_subscriber = FmtSubscriber::builder()
//         .with_env_filter(EnvFilter::from_default_env())
//         .with_ansi(true)
//         .init();

//     let pet = PetSettings {
//         sum: 0.2_f64,
//         update: 0.6_f64,
//         ..Default::default()
//     };

//     let (svc, hdl) = Service::new(pet, MaskSettings::default()).unwrap();
//     let _svc_jh = tokio::spawn(svc);

//     // dummy local model for clients
//     let model = Model::from_primitives(vec![0_f32, 1_f32, 0_f32, 1_f32].into_iter()).unwrap();

//     let mut tasks = vec![];
//     for id in 0..10 {
//         let mut client = Client::new_with_hdl(1, id, hdl.clone())?;
//         client.local_model = Some(model.clone());
//         let join_hdl = tokio::spawn(async move { client.during_round().await });
//         tasks.push(join_hdl);
//     }
//     println!("spawned 10 clients");

//     let mut summers = 0;
//     let mut updaters = 0;
//     let mut unselecteds = 0;
//     for task in tasks {
//         match task.await.or(Err(ClientError::GeneralErr))?? {
//             Task::Update => updaters += 1,
//             Task::Sum => summers += 1,
//             Task::None => unselecteds += 1,
//         }
//     }

//     println!(
//         "{} sum, {} update, {} unselected clients completed a round",
//         summers, updaters, unselecteds
//     );

//     Ok(())
// }
