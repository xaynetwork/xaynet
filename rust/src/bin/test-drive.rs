use xain_fl::client::{Client, ClientError};
use xain_fl::participant::Task;
use xain_fl::service::Service;
use tracing_subscriber::*;


/// Test-drive script of a (completely local) single-round federated learning
/// session, intended for use as a mini integration test. It spawns a Service
/// and 20 Clients on the tokio event loop.
///
/// important NOTE since we only test 20 Clients and by default, the selection
/// ratios in the Coordinator are relatively small, it is very possible no (or
/// too few) Participants will be selected here! It's currently not possible to
/// configure or force the selection, hence as a TEMP workaround, these should
/// be adjusted in coordinator.rs before running this test e.g. 0.2_f64 for sum
/// and 0.5_f64 for update.
#[tokio::main]
async fn main() -> Result<(), ClientError> {

    let _fmt_subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .init();


    let (svc, hdl) = Service::new().unwrap();
    let _svc_jh = tokio::spawn(svc);

    let mut tasks = vec![];
    for id in 0..20 {
        let mut client = Client::new_with_id(1, hdl.clone(), id)?;
        // NOTE give spawn a task that owns client
        // otherwise it won't live long enough
        let join_hdl = tokio::spawn(async move {
            client.per_round().await
        });
        tasks.push(join_hdl);
    }
    println!("spawned 20 clients");

    let mut summers = 0;
    let mut updaters = 0;
    let mut unselecteds = 0;
    for task in tasks {
        match task.await.or(Err(ClientError::GeneralErr))?? {
            Task::Update => updaters    += 1,
            Task::Sum    => summers     += 1,
            Task::None   => unselecteds += 1,
        }
    }

    println!("{} sum, {} update, {} unselected clients completed a round",
             summers, updaters, unselecteds);

    Ok(())
}
