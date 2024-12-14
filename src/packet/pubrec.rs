use crate::packet::*;
use bytes::{BufMut, BytesMut};

#[derive(Debug, Default, Clone)]
pub struct PubRec {
    pub packet_id: u16,
    pub reason_code: ReasonCode,
    pub properties: Option<PubRecProperties>,
}

impl PubRec {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
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
        if version == Version::V5 && (self.reason_code != ReasonCode::Success || props_len > 0) {
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
pub struct PubRecProperties {
    pub reason_string: Option<String>,
    pub user_property: Vec<(String, String)>,
}

impl PubRecProperties {
    pub fn new() -> Self {
        Self {
            ..Default::default()
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
