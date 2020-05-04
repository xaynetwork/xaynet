use xain_fl::client::{Client, ClientError};
use xain_fl::participant::Task;
use xain_fl::service::Service;

#[tokio::main]
async fn main() -> Result<(), ClientError> {

    let (_svc, hdl) = Service::new().unwrap();

    let mut tasks = vec![];
    for _ in 0..50 {
        let mut client = Client::new2(1, hdl.clone())?;
        // NOTE give spawn a task that owns client
        // otherwise it won't live long enough
        let join_hdl = tokio::spawn(async move {
            client.per_round().await
        });
        tasks.push(join_hdl);
    }
    println!("spawned 50 clients");

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
