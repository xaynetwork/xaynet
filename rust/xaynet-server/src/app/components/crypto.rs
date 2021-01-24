pub fn init() -> Result<(), ()> {
    tracing::debug!("initialize");
    sodiumoxide::init()
}
