use crate::packet::*;
use bytes::{Buf, Bytes};

#[derive(Debug, Default, Clone)]
pub struct Subscribe {
    pub packet_id: u16,
    pub properties: Option<SubscribeProperties>,
    pub payload: Vec<Subscription>,
}
#[derive(Debug, Default, Clone)]
pub struct Subscription {
    pub topic: String,
    pub retain_handling: RetainHandling,
    pub retain_as_published: bool,
    pub no_local: bool,
    pub qos: QoS,
}

#[derive(Debug, TryFromPrimitive, Clone)]
#[repr(u8)]
enum RetainHandling {
    Sub = 0,
    NewSub,
    Never,
}
impl Default for RetainHandling {
    fn default() -> Self {
        Self::Sub
    }
}
impl Subscribe {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
    pub fn unpack(mut read: Bytes, version: Version) -> Result<Self, Error> {
        let mut sub = Self::new();

        // Packet ID
        sub.packet_id = read.get_u16();

        // Properties
        if version == Version::V5 {
            sub.properties = SubscribeProperties::unpack(&mut read)?;
        }

        // Payload
        while read.len() > 0 {
            let topic = read_string(&mut read)?;
            let options = read.get_u8();
            let retain_handling = RetainHandling::try_from(options >> 4 & 0x03).unwrap();
            let retain_as_published = options & 0x08 > 0;
            let no_local = options & 0x04 > 0;
            let qos = QoS::try_from(options & 0x03)?;
            sub.payload.push(Subscription {
                topic,
                retain_handling,
                retain_as_published,
                no_local,
                qos,
            })
        }
        Ok(sub)
    }
}

#[derive(Debug, Default, Clone)]
pub struct SubscribeProperties {
    pub sub_identifier: Vec<u32>,
    pub user_property: Vec<(String, String)>,
}

impl SubscribeProperties {
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
