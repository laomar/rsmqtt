use bytes::{BufMut, BytesMut};
use crate::packet::*;

#[derive(Debug, Default)]
pub struct Disconnect {
    pub version: Version,
    pub reason_code: ReasonCode,
    pub properties: Option<DisconnectProperties>,
}

impl Disconnect {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
    pub fn read(mut read: Bytes, version: Version) -> Result<Self, Error> {
        let mut disconnect = Self::new();
        disconnect.version = version;
        if version == Version::V5 {
            disconnect.reason_code = ReasonCode::try_from(read.get_u8()).unwrap();
        }
        Ok(disconnect)
    }

    pub fn write(self, write: &mut BytesMut) -> Result<(), Error> {
        let mut props_len = 0;
        let mut props_buf = BytesMut::with_capacity(512);
        if self.version == Version::V5 {
            if let Some(props) = self.properties {
                props.write(&mut props_buf);
                props_len = props_buf.len();
            }
        }

        let mut buf = BytesMut::with_capacity(512);
        if self.version == Version::V5 {
            buf.put_u8(self.reason_code as u8);
            write_length(&mut buf, props_len)?;
            buf.put(props_buf.freeze());
        }

        write.put_u8((PacketType::Disconnect as u8) << 4);
        write_length(write, buf.len())?;
        write.put(buf.freeze());
        Ok(())
    }
}
#[derive(Debug, Default)]
pub struct DisconnectProperties {
    pub session_expiry_interval: Option<u32>,
    pub server_reference: Option<String>,
    pub reason_string: Option<String>,
    pub user_property: Vec<(String, String)>,
}

impl DisconnectProperties {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
    pub fn read(read: &mut Bytes) -> Result<Option<Self>, Error> {
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
                Property::SessionExpiryInterval => {
                    prop.session_expiry_interval = Some(read.get_u32());
                }

                Property::ServerReference => {
                    prop.server_reference = Some(read_string(&mut read)?);
                }

                Property::ReasonString => {
                    prop.reason_string = Some(read_string(&mut read)?);
                }

                Property::UserProperty => {
                    let k = read_string(&mut read)?;
                    let v = read_string(&mut read)?;
                    prop.user_property.push((k, v));
                }
                _ => return Err(Error::InvalidProperty(format!("0x{identifier:02X}")))
            }
        }
    }

    pub fn write(self, write: &mut BytesMut) {
        if let Some(session_expiry_interval) = self.session_expiry_interval {
            write.put_u8(Property::SessionExpiryInterval as u8);
            write.put_u32(session_expiry_interval);
        }

        if let Some(server_reference) = self.server_reference {
            write.put_u8(Property::ServerReference as u8);
            write_string(write, &server_reference);
        }

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