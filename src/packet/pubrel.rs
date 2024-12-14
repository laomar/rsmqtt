use crate::packet::*;
use bytes::{BufMut, BytesMut};

#[derive(Debug, Default, Clone)]
pub struct PubRel {
    pub packet_id: u16,
    pub reason_code: ReasonCode,
    pub properties: Option<PubRelProperties>,
}

impl PubRel {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
    pub fn unpack(mut read: Bytes, version: Version) -> Result<Self, Error> {
        let mut pubrel = Self::new();
        pubrel.packet_id = read.get_u16();
        if read.len() == 0 {
            return Ok(pubrel);
        }

        if version == Version::V5 {
            pubrel.reason_code = ReasonCode::try_from(read.get_u8())?;
            if read.len() == 0 {
                return Ok(pubrel);
            }
            pubrel.properties = PubRelProperties::unpack(&mut read)?;
        }
        Ok(pubrel)
    }
    pub fn pack(self, write: &mut BytesMut, version: Version) -> Result<(), Error> {
        let mut props_len = 0;
        let mut props_buf = BytesMut::with_capacity(512);
        if let Some(props) = self.properties {
            props.pack(&mut props_buf);
            props_len = props_buf.len();
        }

        let mut buf = BytesMut::with_capacity(512);
        buf.put_u16(self.packet_id);
        if version == Version::V5 {
            buf.put_u8(self.reason_code as u8);
            write_length(&mut buf, props_len)?;
            buf.put(props_buf.freeze());
        }

        write.put_u8((PacketType::PubRec as u8) << 4);
        write_length(write, buf.len())?;
        write.put(buf.freeze());
        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct PubRelProperties {
    pub reason_string: Option<String>,
    pub user_property: Vec<(String, String)>,
}

impl PubRelProperties {
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
                Property::ReasonString => {
                    prop.reason_string = Some(read_string(&mut read)?);
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
    pub fn pack(self, write: &mut BytesMut) {
        if let Some(reason_string) = self.reason_string {
            write.put_u8(Property::ReasonString as u8);
            write_string(write, &reason_string);
        }

        for (k, v) in self.user_property.iter() {
            write.put_u8(Property::UserProperty as u8);
            write_string(write, k);
            write_string(write, v);
        }
    }
}
