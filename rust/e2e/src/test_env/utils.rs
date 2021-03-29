use async_trait::async_trait;
use tokio::time::{interval, Duration};
use xaynet_sdk::{client::Client as HttpApiClient, XaynetClient};
use xaynet_server::state_machine::phases::PhaseName;

use super::influx::InfluxClient;
use crate::utils::terminal::spinner;

#[async_trait]
pub trait IsClientReady {
    async fn is_ready(&mut self) -> bool;
}

pub async fn wait_until_client_is_ready<C: IsClientReady>(client: &mut C) {
    let mut interval = interval(Duration::from_millis(500));
    while !(client.is_ready().await) {
        interval.tick().await;
    }
}

#[async_trait]
impl IsClientReady for InfluxClient {
    async fn is_ready(&mut self) -> bool {
        self.ping().await.is_ok()
    }
}

#[async_trait]
impl IsClientReady for HttpApiClient<reqwest::Client> {
    async fn is_ready(&mut self) -> bool {
        self.get_round_params().await.is_ok()
    }
}

pub async fn wait_until_phase(client: &InfluxClient, phase: PhaseName) {
    let spinner = spinner(&format!("Wait for phase {:?}", phase), "");
    let mut interval = interval(Duration::from_millis(500));
    loop {
        let current_phase = client.get_current_phase().await;
        match current_phase {
            Ok(current_phase) => {
                if current_phase == phase {
                    break;
                } else {
                    spinner.set_message(&format!("current phase: {:?}", current_phase));
                }
            }
            Err(err) => spinner.set_message(&format!("No phase metrics available: {:?}", err)),
        }
        interval.tick().await;
    }
    spinner.finish_with_message("Ok");
}
