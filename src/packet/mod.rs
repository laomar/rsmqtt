use std::io::Error;

trait Packet {}
pub struct FixedHeader {
    packet_type: u8,
    dup: bool,
    qos: u8,
    retain: bool,
    remaining_len: usize,
}

impl FixedHeader {
    pub fn new() -> Self {
        Self {
            packet_type: 0,
            dup: false,
            qos: 0,
            retain: false,
            remaining_len: 0,
        }
    }
    pub fn read(&mut self) -> Result<(), Error> {
        Ok(())
    }

    pub fn write(&mut self) -> Result<(), Error> {
        Ok(())
    }
}
