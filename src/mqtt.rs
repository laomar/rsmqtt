use std::io::Error;
pub struct Mqtt {}
impl Mqtt {
    pub fn new() -> Self {
        Self {}
    }

    pub fn run(mut self) -> Result<(), Error> {
        Ok(())
    }
    pub fn connect() {}
    pub fn publish() {}
    pub fn subscribe() {}
    pub fn unsubscribe() {}
}
