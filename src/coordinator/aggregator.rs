use crate::coordinator::client::ClientId;
pub trait Aggregator<T> {
    // async fn validate_results(&mut self, id: ClientId) -> bool;
    // async fn get_number_of_results(&mut self) -> u32;
    // async fn aggregate(&mut self) -> T;
    fn validate_results(&mut self, id: ClientId) -> bool;
    fn get_number_of_results(&mut self) -> u32;
    fn aggregate(&mut self) -> T;
}
