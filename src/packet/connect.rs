use bytes::{Buf, Bytes};
use crate::packet::*;

// CONNECT Packet
#[derive(Debug, Default)]
pub struct Connect {
    protocol_name: String,
    protocol_version: Version,
    username_flag: bool,
    password_flag: bool,
    will_retain: bool,
    will_qos: QoS,
    will_flag: bool,
    clean_start: bool,
    keep_alive: u16,
    properties: Option<ConnectProperties>,
    client_id: String,
    will_properties: Option<WillProperties>,
    will_topic: String,
    will_payload: String,
    username: String,
    password: String,
}

impl Connect {
    pub fn read(mut packet: Bytes) -> Result<Self, Error> {
        let protocol_name = read_string(&mut packet)?;
        if protocol_name != "MQTT" && protocol_name != "MQIsdp" {
            return Err(Error::InvalidProtocol(protocol_name));
        }
        let version = packet.get_u8();
        let protocol_version = match Version::try_from(version) {
            Ok(v) => v,
            Err(_) => return Err(Error::InvalidProtocolVersion(version)),
        };
        let connect_flags = packet.get_u8();
        let username_flag = connect_flags & 0x80 > 0;
        let password_flag = connect_flags & 0x40 > 0;
        let will_retain = connect_flags & 0x20 > 0;
        let qos = (connect_flags & 0x18) >> 3;
        let will_qos = QoS::try_from(qos).unwrap();
        let will_flag = connect_flags & 0x04 > 0;
        let clean_start = connect_flags & 0x02 > 0;
        let keep_alive = packet.get_u16();
        let mut properties = None;
        if protocol_version == Version::V5 {
            properties = ConnectProperties::read(&mut packet)?;
        }
        let client_id = read_string(&mut packet)?;
        let mut will_properties = None;
        let mut will_topic = "".to_owned();
        let mut will_payload = "".to_owned();
        if will_flag {
            if protocol_version == Version::V5 {
                will_properties = WillProperties::read(&mut packet)?;
            }
            will_topic = read_string(&mut packet)?;
            will_payload = read_string(&mut packet)?;
        }
        let mut username = "".to_owned();
        if username_flag {
            username = read_string(&mut packet)?;
        }
        let mut password = "".to_owned();
        if password_flag {
            password = read_string(&mut packet)?;
        }
        Ok(Connect {
            protocol_name,
            protocol_version,
            username_flag,
            password_flag,
            will_retain,
            will_qos,
            will_flag,
            clean_start,
            keep_alive,
            properties,
            client_id,
            will_properties,
            will_topic,
            will_payload,
            username,
            password,
            ..Default::default()
        })
    }
}

#[derive(Debug, Default)]
pub struct ConnectProperties {
    session_expiry_interval: Option<u32>,
    receive_maximum: Option<u16>,
    max_packet_size: Option<u32>,
    topic_alias_max: Option<u16>,
    request_response_info: Option<u8>,
    request_problem_info: Option<u8>,
    user_property: Vec<(String, String)>,
    auth_method: Option<String>,
    auth_data: Option<Vec<u8>>,
}
impl ConnectProperties {
    pub fn read(read: &mut Bytes) -> Result<Option<Self>, Error> {
        let (mut len, bytes) = read_length(read.iter())?;
        read.advance(bytes);

        if len == 0 {
            return Ok(None);
        }

        let mut read = read.split_to(len);
        let mut prop = Self {
            ..Default::default()
        };

        loop {
            if read.len() == 0 {
                return Ok(Some(prop));
            }
            let identifier = read.get_u8();
            match Property::try_from(identifier)? {
                Property::SessionExpiryInterval => {
                    prop.session_expiry_interval = Some(read.get_u32());
                }
                Property::ReceiveMaximum => {
                    prop.receive_maximum = Some(read.get_u16());
                }
                Property::MaxPacketSize => {
                    prop.max_packet_size = Some(read.get_u32());
                }
                Property::TopicAliasMax => {
                    prop.topic_alias_max = Some(read.get_u16());
                }
                Property::RequestResponseInfo => {
                    prop.request_response_info = Some(read.get_u8());
                }
                Property::RequestProblemInfo => {
                    prop.request_problem_info = Some(read.get_u8());
                }
                Property::UserProperty => {
                    let k = read_string(&mut read)?;
                    let v = read_string(&mut read)?;
                    prop.user_property.push((k, v));
                }
                Property::AuthMethod => {
                    prop.auth_method = Some(read_string(&mut read)?);
                }
                Property::AuthData => {
                    let len = read.get_u16() as usize;
                    let mut read = read.split_to(len);
                    prop.auth_data = Some(read.to_vec())
                }
                _ => return Err(Error::InvalidProperty(format!("0x{:02X}", identifier)))
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct WillProperties {
    content_type: Option<String>,
    response_topic: Option<String>,
    correlation_data: Option<Vec<u8>>,
    will_delay_interval: Option<u32>,
    message_expiry_interval: Option<u32>,
    payload_format_indicator: Option<u8>,
    user_property: Vec<(String, String)>,
}
impl WillProperties {
    pub fn read(read: &mut Bytes) -> Result<Option<Self>, Error> {
        let (mut len, bytes) = read_length(read.iter())?;
        read.advance(bytes);

        if len == 0 {
            return Ok(None);
        }

        let mut read = read.split_to(len);
        let mut prop = Self {
            ..Default::default()
        };

        loop {
            if read.len() == 0 {
                return Ok(Some(prop));
            }
            let identifier = read.get_u8();
            match Property::try_from(identifier)? {
                Property::ContentType => {
                    prop.content_type = Some(read_string(&mut read)?);
                }
                Property::ResponseTopic => {
                    prop.response_topic = Some(read_string(&mut read)?);
                }
                Property::CorrelationData => {
                    let len = read.get_u16() as usize;
                    let mut read = read.split_to(len);
                    prop.correlation_data = Some(read.to_vec())
                }
                Property::WillDelayInterval => {
                    prop.will_delay_interval = Some(read.get_u32());
                }
                Property::MessageExpiryInterval => {
                    prop.message_expiry_interval = Some(read.get_u32());
                }
                Property::PayloadFormatIndicator => {
                    prop.payload_format_indicator = Some(read.get_u8());
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
}
