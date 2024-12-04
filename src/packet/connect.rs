use bytes::{Buf, Bytes};
use crate::packet::*;

// CONNECT Packet
#[derive(Debug, Default)]
pub struct Connect {
    pub protocol_name: String,
    pub protocol_version: Version,
    pub username_flag: bool,
    pub password_flag: bool,
    pub will_retain: bool,
    pub will_qos: QoS,
    pub will_flag: bool,
    pub clean_start: bool,
    pub keep_alive: u16,
    pub properties: Option<ConnectProperties>,
    pub client_id: String,
    pub will_properties: Option<WillProperties>,
    pub will_topic: String,
    pub will_payload: String,
    pub username: String,
    pub password: String,
}

impl Connect {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
    pub fn read(mut read: Bytes) -> Result<Self, Error> {
        let mut connect = Self::new();

        // Protocol Name
        let protocol_name = read_string(&mut read)?;
        if protocol_name != "MQTT" && protocol_name != "MQIsdp" {
            return Err(Error::InvalidProtocol(protocol_name));
        }
        connect.protocol_name = protocol_name;

        // Protocol Version
        let protocol_version = read.get_u8();
        connect.protocol_version = match Version::try_from(protocol_version) {
            Ok(v) => v,
            Err(_) => return Err(Error::InvalidProtocolVersion(protocol_version)),
        };

        // Connect Flags
        let connect_flags = read.get_u8();
        connect.username_flag = connect_flags & 0x80 > 0;
        connect.password_flag = connect_flags & 0x40 > 0;
        connect.will_retain = connect_flags & 0x20 > 0;
        let qos = (connect_flags & 0x18) >> 3;
        connect.will_qos = QoS::try_from(qos).unwrap();
        connect.will_flag = connect_flags & 0x04 > 0;
        connect.clean_start = connect_flags & 0x02 > 0;

        // Keep Alive
        connect.keep_alive = read.get_u16();

        // Properties
        if connect.protocol_version == Version::V5 {
            connect.properties = ConnectProperties::read(&mut read)?;
        }

        // Client ID
        connect.client_id = read_string(&mut read)?;

        // Will
        if connect.will_flag {
            if connect.protocol_version == Version::V5 {
                connect.will_properties = WillProperties::read(&mut read)?;
            }
            connect.will_topic = read_string(&mut read)?;
            connect.will_payload = read_string(&mut read)?;
        }

        // User Name
        if connect.username_flag {
            connect.username = read_string(&mut read)?;
        }

        // Password
        if connect.password_flag {
            connect.password = read_string(&mut read)?;
        }
        Ok(connect)
    }
}

#[derive(Debug, Default)]
pub struct ConnectProperties {
    pub session_expiry_interval: Option<u32>,
    pub receive_maximum: Option<u16>,
    pub max_packet_size: Option<u32>,
    pub topic_alias_max: Option<u16>,
    pub request_response_info: Option<u8>,
    pub request_problem_info: Option<u8>,
    pub user_property: Vec<(String, String)>,
    pub auth_method: Option<String>,
    pub auth_data: Option<Vec<u8>>,
}
impl ConnectProperties {
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
                    let read = read.split_to(len);
                    prop.auth_data = Some(read.to_vec())
                }
                _ => return Err(Error::InvalidProperty(format!("0x{:02X}", identifier)))
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct WillProperties {
    pub content_type: Option<String>,
    pub response_topic: Option<String>,
    pub correlation_data: Option<Vec<u8>>,
    pub will_delay_interval: Option<u32>,
    pub message_expiry_interval: Option<u32>,
    pub payload_format_indicator: Option<u8>,
    pub user_property: Vec<(String, String)>,
}
impl WillProperties {
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
                Property::ContentType => {
                    prop.content_type = Some(read_string(&mut read)?);
                }

                Property::ResponseTopic => {
                    prop.response_topic = Some(read_string(&mut read)?);
                }

                Property::CorrelationData => {
                    let len = read.get_u16() as usize;
                    let read = read.split_to(len);
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
