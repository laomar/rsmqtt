use crate::packet::*;
use bytes::{Buf, Bytes};

#[derive(Debug, Default, Clone)]
pub struct Publish {
    pub dup: bool,
    pub qos: QoS,
    pub retain: bool,
    pub topic_name: String,
    pub packet_id: u16,
    pub properties: Option<PublishProperties>,
    pub payload: Vec<u8>,
}

impl Publish {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
    pub fn unpack(mut read: Bytes, version: Version, byte1: u8) -> Result<Self, Error> {
        let mut publish = Self::new();

        // Fixed Header
        let flags = byte1 & 0x0F;
        publish.dup = flags >> 3 > 0;
        publish.qos = QoS::try_from((flags >> 1) & 0x03)?;
        publish.retain = flags & 0x01 > 0;

        // Topic Name
        publish.topic_name = read_string(&mut read)?;

        // Packet ID
        if publish.qos > QoS::AtMostOnce {
            publish.packet_id = read.get_u16();
        }

        // Properties
        if version == Version::V5 {
            publish.properties = PublishProperties::unpack(&mut read)?;
        }

        // Payload
        publish.payload = read.to_vec();
        Ok(publish)
    }
}

#[derive(Debug, Default, Clone)]
pub struct PublishProperties {
    pub payload_format_indicator: Option<u8>,
    pub message_expiry_interval: Option<u32>,
    pub content_type: Option<String>,
    pub response_topic: Option<String>,
    pub correlation_data: Option<Vec<u8>>,
    pub sub_identifier: Vec<u32>,
    pub topic_alias: Option<u16>,
    pub user_property: Vec<(String, String)>,
}

impl PublishProperties {
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
                Property::PayloadFormatIndicator => {
                    prop.payload_format_indicator = Some(read.get_u8());
                }

                Property::MessageExpiryInterval => {
                    prop.message_expiry_interval = Some(read.get_u32());
                }

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

                Property::SubIdentifier => {
                    let (len, bytes) = read_length(read.iter())?;
                    read.advance(bytes);
                    prop.sub_identifier.push(len as u32);
                }

                Property::TopicAlias => {
                    prop.topic_alias = Some(read.get_u16());
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
