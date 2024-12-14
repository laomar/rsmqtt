use crate::packet::Error::InvalidPacket;
use crate::*;
use bytes::{Buf, BytesMut};

use std::io::ErrorKind;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::timeout;

pub struct Link {
    io: Box<dyn S>,
    read: BytesMut,
    write: BytesMut,
    hook: Arc<Hook>,
    pub version: Version,
    pub client_id: String,
    pub keepalive: Duration,
}
impl Link {
    pub(crate) fn new(io: Box<dyn S>, hook: Arc<Hook>) -> Self {
        Link {
            io,
            hook,
            read: BytesMut::with_capacity(10 * 1024),
            write: BytesMut::with_capacity(10 * 1024),
            version: Version::default(),
            client_id: Default::default(),
            keepalive: Duration::from_secs(5),
        }
    }

    fn set_keepalive(&mut self, keepalive: u16) {
        let keepalive = (keepalive as f32 * 1.5).ceil() as u64;
        self.keepalive = Duration::from_secs(keepalive);
    }
    async fn read_bytes(&mut self, n: usize) -> Result<usize, Error> {
        let mut count = 0;
        loop {
            let read = self.io.read_buf(&mut self.read).await?;
            if read == 0 {
                return Err(Error::Io(io::Error::new(
                    ErrorKind::ConnectionReset,
                    "connection closed by peer",
                )));
            }
            count += read;
            if count >= n {
                return Ok(count);
            }
        }
    }

    async fn read_packet(&mut self) -> Result<Packet, Error> {
        let mut n = 2;
        loop {
            timeout(self.keepalive, self.read_bytes(n)).await??;

            // println!(
            //     "{}",
            //     self.read
            //         .to_vec()
            //         .iter()
            //         .map(|b| format!("{:02X}", b))
            //         .collect::<Vec<String>>()
            //         .join(" ")
            // );

            let mut read = self.read.iter();
            let byte1 = *read.next().unwrap();
            let (remaining_len, bytes) = match read_length(read) {
                Ok((l, b)) => (l, b),
                Err(_) => {
                    n = 1;
                    continue;
                }
            };

            let len = 1 + bytes + remaining_len;
            if len > self.read.len() {
                n = len - self.read.len();
                continue;
            }

            let mut packet = self.read.split_to(len).freeze();
            if remaining_len > 0 {
                packet.advance(1 + bytes);
            }

            let packet = match PacketType::try_from(byte1 >> 4)? {
                PacketType::Connect => {
                    let connect = Connect::unpack(packet)?;
                    Packet::Connect(connect)
                }
                PacketType::Publish => {
                    let publish = Publish::unpack(packet, self.version, byte1)?;
                    Packet::Publish(publish)
                }
                PacketType::PubRel => {
                    let pubrel = PubRel::unpack(packet, self.version)?;
                    Packet::PubRel(pubrel)
                }
                PacketType::Subscribe => {
                    let subscribe = Subscribe::unpack(packet, self.version)?;
                    Packet::Subscribe(subscribe)
                }
                PacketType::Unsubscribe => {
                    let unsubscribe = Unsubscribe::unpack(packet, self.version)?;
                    Packet::Unsubscribe(unsubscribe)
                }
                PacketType::PingReq => Packet::PingReq,
                PacketType::Disconnect => {
                    let disconnect = Disconnect::unpack(packet, self.version)?;
                    Packet::Disconnect(disconnect)
                }
                PacketType::Auth => {
                    let auth = Auth::unpack(packet)?;
                    Packet::Auth(auth)
                }
                _ => {
                    return Err(Error::Packet(InvalidPacket(format!(
                        "0x{:02X}",
                        byte1 >> 4
                    ))))
                }
            };

            return Ok(packet);
        }
    }

    async fn write_packet(&mut self, packet: Packet) -> Result<(), Error> {
        match packet {
            Packet::ConnAck(connack) => {
                //println!("{:?}", connack);
                connack.pack(&mut self.write, self.version)?;
            }
            Packet::PingResp => {
                pingresp::pack(&mut self.write);
            }
            Packet::Disconnect(disconnect) => {
                //println!("{:?}", disconnect);
                disconnect.pack(&mut self.write, self.version)?;
            }
            Packet::PubAck(puback) => {
                //println!("{:?}", puback);
                puback.pack(&mut self.write, self.version)?;
            }
            Packet::PubRec(pubrec) => {
                //println!("{:?}", pubrec);
                pubrec.pack(&mut self.write, self.version)?;
            }
            Packet::PubComp(pubcomp) => {
                //println!("{:?}", pubcomp);
                pubcomp.pack(&mut self.write, self.version)?;
            }
            Packet::SubAck(suback) => {
                //println!("{:?}", suback);
                suback.pack(&mut self.write, self.version)?;
            }
            Packet::UnsubAck(unsuback) => {
                //println!("{:?}", unsuback);
                unsuback.pack(&mut self.write, self.version)?;
            }
            Packet::Auth(auth) => {
                //println!("{:?}", auth);
                auth.pack(&mut self.write)?;
            }
            _ => unreachable!(),
        }
        self.io.write_all(&self.write).await?;
        self.write.clear();
        Ok(())
    }
    pub async fn serve(mut self) {
        match self.connect().await {
            Ok(_) => loop {
                let packet = match self.read_packet().await {
                    Ok(p) => p,
                    Err(e) => {
                        println!("{}: {}", self.client_id, e);
                        return;
                    }
                };
                let r = self.hook.trigger(packet.clone());
                println!("{:?}", r);
                match packet {
                    Packet::PingReq => {
                        //println!("{}: {:?}", self.client_id, packet);
                        self.write_packet(Packet::PingResp).await.unwrap();
                    }
                    Packet::Publish(publish) => match publish.qos {
                        QoS::AtMostOnce => {}
                        QoS::AtLeastOnce => {
                            let mut puback = PubAck::new();
                            puback.packet_id = publish.packet_id;
                            self.write_packet(Packet::PubAck(puback)).await.unwrap()
                        }
                        QoS::ExactlyOnce => {
                            let mut pubrec = PubRec::new();
                            pubrec.packet_id = publish.packet_id;
                            self.write_packet(Packet::PubRec(pubrec)).await.unwrap()
                        }
                    },
                    Packet::PubRel(pubrel) => {
                        println!("{}: {:?}", self.client_id, pubrel);
                        let mut pubcomp = PubComp::new();
                        pubcomp.packet_id = pubrel.packet_id;
                        self.write_packet(Packet::PubComp(pubcomp)).await.unwrap()
                    }
                    Packet::Subscribe(subscribe) => {
                        println!("{}: {:?}", self.client_id, subscribe);
                        let mut suback = SubAck::new();
                        suback.packet_id = subscribe.packet_id;
                        suback.payload.push(ReasonCode::Success);
                        self.write_packet(Packet::SubAck(suback)).await.unwrap()
                    }
                    Packet::Unsubscribe(unsubscribe) => {
                        println!("{}: {:?}", self.client_id, unsubscribe);
                        let mut unsuback = UnsubAck::new();
                        unsuback.packet_id = unsubscribe.packet_id;
                        unsuback.payload.push(ReasonCode::Success);
                        self.write_packet(Packet::UnsubAck(unsuback)).await.unwrap()
                    }
                    Packet::Disconnect(disconnect) => {
                        println!("{}: {:?}", self.client_id, disconnect);
                        break;
                    }
                    _ => {}
                }
            },
            Err(e) => {
                println!("{e}")
            }
        }
    }

    async fn connect(&mut self) -> Result<bool, Error> {
        let packet = match self.read_packet().await {
            Ok(p) => p,
            Err(e) => return Err(e),
        };
        let Packet::Connect(connect) = packet else {
            return Err(Error::NotConnectPacket);
        };

        //println!("{:?}", connect);

        //let _ = self.stream.shutdown().await;
        self.version = connect.protocol_version;
        self.client_id = connect.client_id.clone();
        self.set_keepalive(connect.keepalive);

        //let client = Arc::new(self);
        let r = self.hook.trigger(Packet::Connect(connect));
        println!("{:?}", r);

        self.ack().await?;
        Ok(true)
    }

    async fn ack(&mut self) -> Result<(), Error> {
        let mut reason_code = ReasonCode::Success;
        let mut ack = ConnAck::new();
        ack.reason_code = reason_code;
        //let mut prop = ConnAckProperties::new();
        //prop.topic_alias_max = Some(300);
        //ack.properties = Some(prop);
        let packet = Packet::ConnAck(ack);
        self.write_packet(packet).await?;
        Ok(())
    }
}
