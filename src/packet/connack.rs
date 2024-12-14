use crate::packet::*;
use bytes::{BufMut, BytesMut};

#[derive(Debug, Default, Clone)]
pub struct ConnAck {
    pub session_present: bool,
    pub reason_code: ReasonCode,
    pub properties: Option<ConnAckProperties>,
}

impl ConnAck {
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
        buf.put_u8(self.session_present as u8);
        buf.put_u8(self.reason_code as u8);
        if version == Version::V5 {
            write_length(&mut buf, props_len)?;
            buf.put(props_buf.freeze());
        }

        write.put_u8((PacketType::ConnAck as u8) << 4);
        write_length(write, buf.len())?;
        write.put(buf.freeze());
        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct ConnAckProperties {
    pub session_expiry_interval: Option<u32>,
    pub assigned_client_identifier: Option<String>,
    pub server_keep_alive: Option<u16>,
    pub auth_method: Option<String>,
    pub auth_data: Option<Vec<u8>>,
    pub response_info: Option<String>,
    pub server_reference: Option<String>,
    pub reason_string: Option<String>,
    pub receive_maximum: Option<u16>,
    pub topic_alias_max: Option<u16>,
    pub maximum_qos: Option<u8>,
    pub retain_available: Option<u8>,
    pub user_property: Vec<(String, String)>,
    pub max_packet_size: Option<u32>,
    pub wildcard_sub_available: Option<u8>,
    pub sub_identifier_available: Option<u8>,
    pub shared_sub_available: Option<u8>,
}
impl ConnAckProperties {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
    pub fn pack(self, write: &mut BytesMut) {
        if let Some(session_expiry_interval) = self.session_expiry_interval {
            write.put_u8(Property::SessionExpiryInterval as u8);
            write.put_u32(session_expiry_interval);
        }

        if let Some(assigned_client_identifier) = self.assigned_client_identifier {
            write.put_u8(Property::AssignedClientIdentifier as u8);
            write_string(write, &assigned_client_identifier);
        }

        if let Some(server_keep_alive) = self.server_keep_alive {
            write.put_u8(Property::ServerKeepAlive as u8);
            write.put_u16(server_keep_alive);
        }

        if let Some(auth_method) = self.auth_method {
            write.put_u8(Property::AuthMethod as u8);
            write_string(write, &auth_method);
        }

        if let Some(auth_data) = self.auth_data {
            write.put_u8(Property::AuthData as u8);
            write.put_u16(auth_data.len() as u16);
            write.extend_from_slice(&auth_data);
        }

        if let Some(response_info) = self.response_info {
            write.put_u8(Property::ResponseInfo as u8);
            write_string(write, &response_info);
        }

        if let Some(server_reference) = self.server_reference {
            write.put_u8(Property::ServerReference as u8);
            write_string(write, &server_reference);
        }

        if let Some(reason_string) = self.reason_string {
            write.put_u8(Property::ReasonString as u8);
            write_string(write, &reason_string);
        }

        if let Some(receive_maximum) = self.receive_maximum {
            write.put_u8(Property::ReceiveMaximum as u8);
            write.put_u16(receive_maximum);
        }

        if let Some(topic_alias_max) = self.topic_alias_max {
            write.put_u8(Property::TopicAliasMax as u8);
            write.put_u16(topic_alias_max);
        }

        if let Some(maximum_qos) = self.maximum_qos {
            write.put_u8(Property::MaximumQoS as u8);
            write.put_u8(maximum_qos);
        }

        if let Some(retain_available) = self.retain_available {
            write.put_u8(Property::RetainAvailable as u8);
            write.put_u8(retain_available);
        }

        for (k, v) in self.user_property.iter() {
            write.put_u8(Property::UserProperty as u8);
            write_string(write, k);
            write_string(write, v);
        }

        if let Some(max_packet_size) = self.max_packet_size {
            write.put_u8(Property::MaxPacketSize as u8);
            write.put_u32(max_packet_size);
        }

        if let Some(wildcard_sub_available) = self.wildcard_sub_available {
            write.put_u8(Property::WildcardSubAvailable as u8);
            write.put_u8(wildcard_sub_available);
        }

        if let Some(sub_identifier_available) = self.sub_identifier_available {
            write.put_u8(Property::SubIdentifierAvailable as u8);
            write.put_u8(sub_identifier_available);
        }

        if let Some(shared_sub_available) = self.shared_sub_available {
            write.put_u8(Property::SharedSubAvailable as u8);
            write.put_u8(shared_sub_available);
        }
    }
}
