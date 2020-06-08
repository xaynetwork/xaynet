use reqwest::Client;
use reqwest::Response;
use reqwest::Error;

pub struct PetHttp {
    client: Client,
    address: &'static str,
}

impl PetHttp {

    pub fn new(address: &'static str) -> Self {
        PetHttp {
            client: Client::new(),
            address,
        }
    }

    pub async fn get_sums(&self) {
        // TODO append path
        let response: Response = self.client.get(self.address).send().await?;
        let _bytes = response.bytes().await?;
    }
}
