mod auth;
mod connack;
mod connect;
mod disconnect;
mod ping;
mod puback;
mod pubcomp;
mod publish;
mod pubrec;
mod pubrel;
mod suback;
mod subscribe;
mod unsuback;
mod unsubscribe;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use num_enum::{TryFromPrimitive, TryFromPrimitiveError};
use std::slice::Iter;
use std::string::FromUtf8Error;

pub use auth::*;
pub use connack::*;
pub use connect::*;
pub use disconnect::*;
pub use ping::*;
pub use puback::*;
pub use pubcomp::*;
pub use publish::*;
pub use pubrec::*;
pub use pubrel::*;
pub use suback::*;
pub use subscribe::*;
pub use unsuback::*;
pub use unsubscribe::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("String is not utf8: {0}")]
    NotUtf8(#[from] FromUtf8Error),
    #[error("Packet is too short")]
    PacketTooShort,
    #[error("Payload is too long")]
    PayloadTooLong,
    #[error("Invalid packet: {0}")]
    InvalidPacket(String),
    #[error("Invalid protocol: {0}")]
    InvalidProtocol(String),
    #[error("Invalid protocol version: {0}")]
    InvalidProtocolVersion(u8),
    #[error("Invalid property: {0}")]
    TryFromProperty(#[from] TryFromPrimitiveError<Property>),
    #[error("Invalid reason code: {0}")]
    TryFromReasonCode(#[from] TryFromPrimitiveError<ReasonCode>),
    #[error("Invalid QoS: {0}")]
    TryFromQoS(#[from] TryFromPrimitiveError<QoS>),
}
#[derive(Debug, TryFromPrimitive)]
#[repr(u8)]
pub enum PacketType {
    Reserved = 0,
    Connect,
    ConnAck,
    Publish,
    PubAck,
    PubRec,
    PubRel,
    PubComp,
    Subscribe,
    SubAck,
    Unsubscribe,
    UnsubAck,
    PingReq,
    PingResp,
    Disconnect,
    Auth,
}
#[derive(Debug, Clone)]
pub enum Packet {
    Connect(Connect),
    ConnAck(ConnAck),
    Publish(Publish),
    PubAck(PubAck),
    PubRec(PubRec),
    PubRel(PubRel),
    PubComp(PubComp),
    Subscribe(Subscribe),
    SubAck(SubAck),
    Unsubscribe(Unsubscribe),
    UnsubAck(UnsubAck),
    PingReq,
    PingResp,
    Disconnect(Disconnect),
    Auth(Auth),
    None,
}
#[derive(Debug, PartialEq, PartialOrd, Clone, TryFromPrimitive)]
#[repr(u8)]
pub enum QoS {
    AtMostOnce = 0,
    AtLeastOnce,
    ExactlyOnce,
}
impl Default for QoS {
    fn default() -> Self {
        Self::AtMostOnce
    }
}
#[derive(Debug, PartialEq, Copy, Clone, TryFromPrimitive)]
#[repr(u8)]
pub enum Version {
    V31 = 3,
    V311,
    V5,
}
impl Default for Version {
    fn default() -> Self {
        Self::V5
    }
}
#[derive(Debug, TryFromPrimitive)]
#[repr(u8)]
pub enum Property {
    PayloadFormatIndicator = 0x01,
    MessageExpiryInterval = 0x02,
    ContentType = 0x03,
    ResponseTopic = 0x08,
    CorrelationData = 0x09,
    SubIdentifier = 0x0B,
    SessionExpiryInterval = 0x11,
    AssignedClientIdentifier = 0x12,
    ServerKeepAlive = 0x13,
    AuthMethod = 0x15,
    AuthData = 0x16,
    RequestProblemInfo = 0x17,
    WillDelayInterval = 0x18,
    RequestResponseInfo = 0x19,
    ResponseInfo = 0x1A,
    ServerReference = 0x1C,
    ReasonString = 0x1F,
    ReceiveMaximum = 0x21,
    TopicAliasMax = 0x22,
    TopicAlias = 0x23,
    MaximumQoS = 0x24,
    RetainAvailable = 0x25,
    UserProperty = 0x26,
    MaxPacketSize = 0x27,
    WildcardSubAvailable = 0x28,
    SubIdentifierAvailable = 0x29,
    SharedSubAvailable = 0x2A,
}
#[derive(Debug, PartialEq, PartialOrd, Copy, Clone, TryFromPrimitive)]
#[repr(u8)]
pub enum ReasonCode {
    Success = 0x00,
    GrantedQoS1 = 0x01,
    GrantedQoS2 = 0x02,
    DisconnectWithWillMessage = 0x04,
    NotMatchingSubscribers = 0x10,
    NoSubscriptionExisted = 0x11,
    ContinueAuthentication = 0x18,
    ReAuthenticate = 0x19,
    UnspecifiedError = 0x80,
    MalformedPacket = 0x81,
    ProtocolError = 0x82,
    ImplementationSpecificError = 0x83,
    UnsupportedProtocolVersion = 0x84,
    ClientIdentifierNotValid = 0x85,
    BadUserNameOrPassword = 0x86,
    NotAuthorized = 0x87,
    ServerUnavailable = 0x88,
    ServerBusy = 0x89,
    Banned = 0x8A,
    ServerShuttingDown = 0x8B,
    BadAuthMethod = 0x8C,
    KeepAliveTimeout = 0x8D,
    SessionTakenOver = 0x8E,
    TopicFilterInvalid = 0x8F,
    TopicNameInvalid = 0x90,
    PacketIDInUse = 0x91,
    PacketIDNotFound = 0x92,
    RecvMaxExceeded = 0x93,
    TopicAliasInvalid = 0x94,
    PacketTooLarge = 0x95,
    MessageRateTooHigh = 0x96,
    QuotaExceeded = 0x97,
    AdminAction = 0x98,
    PayloadFormatInvalid = 0x99,
    RetainNotSupported = 0x9A,
    QoSNotSupported = 0x9B,
    UseAnotherServer = 0x9C,
    ServerMoved = 0x9D,
    SharedSubNotSupported = 0x9E,
    ConnectionRateExceeded = 0x9F,
    MaxConnectTime = 0xA0,
    SubIDNotSupported = 0xA1,
    WildcardSubNotSupported = 0xA2,
}
impl Default for ReasonCode {
    fn default() -> Self {
        Self::Success
    }
}

fn read_string(read: &mut Bytes) -> Result<String, Error> {
    let len = read.get_u16() as usize;
    if len > read.len() {
        return Err(Error::PacketTooShort);
    }
    let bytes = read.split_to(len);
    let str = String::from_utf8(bytes.to_vec())?;
    Ok(str)
}
fn write_string(write: &mut BytesMut, str: &str) {
    write.put_u16(str.len() as u16);
    write.extend_from_slice(str.as_bytes());
}

pub fn read_length(read: Iter<u8>) -> Result<(usize, usize), Error> {
    let mut len = 0;
    let mut bytes = 0;
    let mut mul = 0;
    let mut done = false;
    for byte in read {
        bytes += 1;
        let byte = *byte as usize;
        len |= (byte & 0x7F) << mul;
        if byte & 0x80 == 0 {
            done = true;
            break;
        }
        mul += 7;
    }

    if !done {
        return Err(Error::PacketTooShort);
    }
    Ok((len, bytes))
}

fn write_length(write: &mut BytesMut, mut len: usize) -> Result<(), Error> {
    if len > 268_435_455 {
        return Err(Error::PayloadTooLong);
    }
    loop {
        let mut byte = (len % 128) as u8;
        len /= 128;
        if len > 0 {
            byte |= 0x80;
        }
        write.put_u8(byte);
        if len <= 0 {
            return Ok(());
        }
    }
}
