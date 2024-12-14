use crate::packet::*;
use bytes::{Buf, Bytes};

#[derive(Debug, Default, Clone)]
pub struct Unsubscribe {
    pub packet_id: u16,
    pub properties: Option<UnsubscribeProperties>,
    pub payload: Vec<String>,
}

impl Unsubscribe {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
    pub fn unpack(mut read: Bytes, version: Version) -> Result<Self, Error> {
        let mut unsub = Self::new();

        // Packet ID
        unsub.packet_id = read.get_u16();

        // Properties
        if version == Version::V5 {
            unsub.properties = UnsubscribeProperties::unpack(&mut read)?;
        }

        // Payload
        while read.len() > 0 {
            let topic = read_string(&mut read)?;
            unsub.payload.push(topic);
        }
        Ok(unsub)
    }
}

#[derive(Debug, Default, Clone)]
pub struct UnsubscribeProperties {
    pub sub_identifier: Vec<u32>,
    pub user_property: Vec<(String, String)>,
}

impl UnsubscribeProperties {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn unpack(read: &mut Bytes) -> Result<Option<Self>, Error> {
        let (len, bytes) = read_length(read.iter())?;
        read.advance(bytes);

        if len == 0 {
            return Ok(None);
        }

        let mut read = read.split_to(len);
        let mut prop = Self::new();

        loop {
            if read.len() == 0 {
                return Ok(Some(prop));
            }
            let identifier = read.get_u8();
            match Property::try_from(identifier)? {
                Property::SubIdentifier => {
                    let (len, bytes) = read_length(read.iter())?;
                    read.advance(bytes);
                    prop.sub_identifier.push(len as u32);
                }

                Property::UserProperty => {
                    let k = read_string(&mut read)?;
                    let v = read_string(&mut read)?;
                    prop.user_property.push((k, v));
                }
                _ => unreachable!(),
            }
        }
    }
}
