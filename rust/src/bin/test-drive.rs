use xain_fl::client::{Client, ClientError};
use xain_fl::participant::Task;
use xain_fl::service::Service;
use tracing_subscriber::*;


#[tokio::main]
async fn main() -> Result<(), ClientError> {
    let _fmt_subscriber = FmtSubscriber::builder()
        .with_ansi(true)
        .init();


    let (svc, hdl) = Service::new().unwrap();
    let _svc_jh = tokio::spawn(svc);

    let mut tasks = vec![];
    for id in 0..20 {
        let mut client = Client::new2(1, hdl.clone(), id)?;
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
