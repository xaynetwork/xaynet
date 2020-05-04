use xain_fl::client::{Client, ClientError};
use xain_fl::participant::Task;
use xain_fl::service::Service;

#[tokio::main]
async fn main() -> Result<(), ClientError> {
    // create a largeish number of clients
    // do this in a loop: in each iteration,
    // create a client c; spawn a task that runs c.per_round
    // wait for them to finish
    // if they all returned () ... that's as good as it gets for now
    // error case easier to test: spawn too few
    // "inbetween": dropouts

    // tweak probabilities before actually running this
    // print statements somehow would be nice
    // later on it would be good to actually print a value to prove it's working
    // e.g the global model

//    let mut c = Client::new(1)?;
//    let jh = tokio::spawn(c.per_round());

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
