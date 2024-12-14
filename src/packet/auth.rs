use crate::packet::*;
use bytes::{BufMut, BytesMut};

#[derive(Debug, Default, Clone)]
pub struct Auth {
    pub reason_code: ReasonCode,
    pub properties: Option<AuthProperties>,
}

impl Auth {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn unpack(mut read: Bytes) -> Result<Self, Error> {
        let mut auth = Self::new();
        auth.reason_code = ReasonCode::try_from(read.get_u8())?;
        auth.properties = AuthProperties::unpack(&mut read)?;
        Ok(auth)
    }
    pub fn pack(self, write: &mut BytesMut) -> Result<(), Error> {
        let mut props_len = 0;
        let mut props_buf = BytesMut::with_capacity(512);
        if let Some(props) = self.properties {
            props.pack(&mut props_buf);
            props_len = props_buf.len();
        }

        let mut buf = BytesMut::with_capacity(512);
        buf.put_u8(self.reason_code as u8);
        write_length(&mut buf, props_len)?;
        buf.put(props_buf.freeze());

        write.put_u8((PacketType::Auth as u8) << 4);
        write_length(write, buf.len())?;
        write.put(buf.freeze());
        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct AuthProperties {
    pub auth_method: Option<String>,
    pub auth_data: Option<Vec<u8>>,
    pub reason_string: Option<String>,
    pub user_property: Vec<(String, String)>,
}

impl AuthProperties {
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
                Property::AuthMethod => {
                    prop.auth_method = Some(read_string(&mut read)?);
                }

                Property::AuthData => {
                    let len = read.get_u16() as usize;
                    let read = read.split_to(len);
                    prop.auth_data = Some(read.to_vec())
                }

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
        if let Some(auth_method) = self.auth_method {
            write.put_u8(Property::AuthMethod as u8);
            write_string(write, &auth_method);
        }

        if let Some(auth_data) = self.auth_data {
            write.put_u8(Property::AuthData as u8);
            write.put_u16(auth_data.len() as u16);
            write.put_slice(&auth_data);
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
