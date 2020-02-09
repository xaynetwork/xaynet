pub trait Aggregator<T> {
    type Error: ::std::error::Error;

    fn add_local_result(&mut self, result: T) -> Result<(), Self::Error>;
    fn aggregate(&mut self) -> Result<T, Self::Error>;
}
