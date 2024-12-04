use crate::hook::{Hook, HookFn, HookType};
use async_tungstenite::tokio::accept_hdr_async;
use async_tungstenite::tungstenite::handshake::server::{
    Callback, ErrorResponse, Request, Response,
};
use async_tungstenite::tungstenite::http::HeaderValue;
use thiserror::Error;
use std::fmt::Debug;
use std::{io, time};
use std::io::ErrorKind;
use std::sync::{Arc, RwLock};
use bytes::{Buf, BytesMut};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task;
use tokio::time::error::Elapsed;
use tokio::time::timeout;
use tokio_rustls::rustls::Error as RustlsError;
use tokio_rustls::rustls::pki_types::pem::PemObject;
use tokio_rustls::rustls::pki_types::pem::Error as PemError;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::{TlsAcceptor};
use ws_stream_tungstenite::WsStream;
use crate::packet::{self, *};

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Keepalive timeout: {0}")]
    Timeout(#[from] Elapsed),
    #[error("Pem error: {0}")]
    Pem(#[from] PemError),
    #[error("Rustls error: {0}")]
    Rustls(#[from] RustlsError),
    #[error("Packet error: {0}")]
    Packet(#[from] packet::Error),
    #[error("Not connect packet")]
    NotConnectPacket,
}

pub struct Mqtt {
    listens: Vec<Listen>,
    hooks: Arc<RwLock<Vec<Hook>>>,
    proxy_protocol: bool,
}
impl Mqtt {
    pub fn new() -> Self {
        Self {
            listens: Vec::new(),
            hooks: Arc::new(RwLock::new(Vec::new())),
            proxy_protocol: false,
        }
    }
    pub fn listen(&mut self, protocol: &str, addr: &str) -> &mut Self {
        self.listens(protocol, addr, "", "");
        self
    }
    pub fn listens(&mut self, protocol: &str, addr: &str, cert: &str, key: &str) -> &mut Self {
        self.listens.push(Listen {
            protocol: protocol.to_owned(),
            addr: addr.to_owned(),
            cert: cert.to_owned(),
            key: key.to_owned(),
        });
        self
    }
    pub fn hook(&mut self, ht: HookType) -> &mut Self {
        self.hooks.write().unwrap().push(Hook {});
        self
    }
    pub async fn run(&mut self) -> Result<(), Error> {
        if self.listens.is_empty() {
            self.listen("tcp", "0.0.0.0:1883");
        }
        for listen in self.listens.clone() {
            let hooks = self.hooks.clone();
            task::spawn(async move {
                if let Err(e) = listen.serve(hooks).await {
                    println!("{}", e);
                }
            });
        }
        Ok(())
    }
}
#[derive(Debug, Clone)]
struct Listen {
    protocol: String,
    addr: String,
    cert: String,
    key: String,
}
impl Listen {
    async fn serve(&self, hooks: Arc<RwLock<Vec<Hook>>>) -> Result<(), Error> {
        let protocol = self.protocol.as_str();
        println!("{}", protocol);
        let acceptor = match protocol {
            "tls" | "wss" => Some(self.acceptor()?),
            _ => None
        };
        let listener = TcpListener::bind(&self.addr).await?;
        loop {
            let stream = match listener.accept().await {
                Ok((stream, addr)) => {
                    println!("{:?}", addr);
                    stream
                }
                Err(_) => continue
            };
            match protocol {
                "tcp" => {
                    task::spawn(Client::new(stream).serve());
                }
                "tls" => {
                    let acceptor = acceptor.clone().unwrap();
                    let stream = match acceptor.accept(stream).await {
                        Ok(stream) => stream,
                        Err(_) => continue
                    };
                    task::spawn(Client::new(stream).serve());
                }
                "ws" => {
                    let stream = match accept_hdr_async(stream, WSCallback).await {
                        Ok(stream) => stream,
                        Err(_) => continue
                    };
                    let stream = WsStream::new(stream);
                    task::spawn(Client::new(stream).serve());
                }
                "wss" => {
                    let acceptor = acceptor.clone().unwrap();
                    let stream = match acceptor.accept(stream).await {
                        Ok(stream) => stream,
                        Err(_) => continue
                    };
                    let stream = match accept_hdr_async(stream, WSCallback).await {
                        Ok(stream) => stream,
                        Err(_) => continue
                    };
                    let stream = WsStream::new(stream);
                    task::spawn(Client::new(stream).serve());
                }
                _ => (),
            }
        }
    }
    fn acceptor(&self) -> Result<TlsAcceptor, Error> {
        let key = PrivateKeyDer::from_pem_file(&self.key)?;
        let certs =
            CertificateDer::pem_file_iter(&self.cert)?.collect::<Result<Vec<_>, _>>()?;
        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;
        let acceptor = TlsAcceptor::from(Arc::new(config));
        Ok(acceptor)
    }
}
struct WSCallback;
impl Callback for WSCallback {
    fn on_request(
        self,
        _request: &Request,
        mut response: Response,
    ) -> Result<Response, ErrorResponse> {
        response
            .headers_mut()
            .insert("sec-websocket-protocol", HeaderValue::from_static("mqtt"));
        Ok(response)
    }
}

trait S: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T> S for T where T: AsyncRead + AsyncWrite + Unpin + Send {}
struct Client<T> {
    stream: T,
    version: Version,
    read: BytesMut,
    write: BytesMut,
}
impl<T: S + Debug> Client<T> {
    fn new(stream: T) -> Self {
        Client {
            stream,
            version: Version::V5,
            read: BytesMut::with_capacity(10 * 1024),
            write: BytesMut::with_capacity(10 * 1024),
        }
    }
    async fn read_bytes(&mut self, n: usize) -> io::Result<usize> {
        let mut count = 0;
        loop {
            let read = self.stream.read_buf(&mut self.read).await?;
            if read == 0 {
                return Err(io::Error::new(ErrorKind::ConnectionReset, "connection closed by peer"));
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
            self.read_bytes(n).await?;
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
            n = len - self.read.len();
            if n > 0 {
                continue;
            }

            let mut packet = self.read.split_to(len).freeze();
            if remaining_len > 0 {
                packet.advance(1 + bytes);
            }

            let packet = match PacketType::try_from(byte1 >> 4).unwrap() {
                PacketType::Connect => {
                    let connect = Connect::read(packet)?;
                    Packet::Connect(connect)
                }
                PacketType::Publish => {
                    let publish = Publish::read(packet, self.version, byte1)?;
                    Packet::Publish(publish)
                }
                PacketType::PubRel => {
                    let pubrel = PubRel::read(packet, self.version)?;
                    Packet::PubRel(pubrel)
                }
                PacketType::Subscribe => {
                    let subscribe = Subscribe::read(packet, self.version)?;
                    Packet::Subscribe(subscribe)
                }
                // PacketType::Unsubscribe => {}
                PacketType::PingReq => {
                    Packet::PingReq
                }
                PacketType::Disconnect => {
                    let disconnect = Disconnect::read(packet, self.version)?;
                    Packet::Disconnect(disconnect)
                }
                // PacketType::Auth => {}
                _ => unreachable!()
            };


            return Ok(packet);
        }
    }

    async fn write_packet(&mut self, packet: Packet) -> Result<(), Error> {
        match packet {
            Packet::ConnAck(connack) => {
                println!("{:?}", connack);
                connack.write(&mut self.write, self.version)?;
            }
            Packet::PingResp => {
                pingresp::write(&mut self.write);
            }
            Packet::Disconnect(disconnect) => {
                println!("{:?}", disconnect);
                disconnect.write(&mut self.write, self.version)?;
            }
            Packet::PubAck(puback) => {
                println!("{:?}", puback);
                puback.write(&mut self.write, self.version)?;
            }
            Packet::PubRec(pubrec) => {
                println!("{:?}", pubrec);
                pubrec.write(&mut self.write, self.version)?;
            }
            Packet::PubComp(pubcomp) => {
                println!("{:?}", pubcomp);
                pubcomp.write(&mut self.write, self.version)?;
            }
            Packet::SubAck(suback) => {
                println!("{:?}", suback);
                suback.write(&mut self.write, self.version)?;
            }
            _ => unreachable!()
        }
        self.stream.write_all(&self.write).await?;
        self.write.clear();
        Ok(())
    }
    async fn serve(mut self) {
        match self.connect().await {
            Ok(_) => {
                loop {
                    let packet = match self.read_packet().await {
                        Ok(p) => p,
                        Err(_) => break
                    };
                    match packet {
                        Packet::PingReq => {
                            println!("{:?}", packet);
                            self.write_packet(Packet::PingResp).await.unwrap();
                        }
                        Packet::Publish(publish) => {
                            println!("{:?}", publish);
                            match publish.qos {
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
                            }
                        }
                        Packet::PubRel(pubrel) => {
                            println!("{:?}", pubrel);
                            let mut pubcomp = PubComp::new();
                            pubcomp.packet_id = pubrel.packet_id;
                            self.write_packet(Packet::PubComp(pubcomp)).await.unwrap()
                        }
                        Packet::Subscribe(subscribe) => {
                            println!("{:?}", subscribe);
                            let mut suback = SubAck::new();
                            suback.packet_id = subscribe.packet_id;
                            suback.payload.push(0x00);
                            self.write_packet(Packet::SubAck(suback)).await.unwrap()
                        }
                        Packet::Disconnect(disconnect) => {
                            println!("{:?}", disconnect);
                            break;
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => {
                println!("{e}")
            }
        }
    }

    async fn connect(&mut self) -> Result<bool, Error> {
        let packet = timeout(time::Duration::from_secs(5), self.read_packet()).await??;
        let connect = match packet {
            Packet::Connect(connect) => connect,
            _ => return Err(Error::NotConnectPacket),
        };

        println!("{:?}", connect);
        self.version = connect.protocol_version;

        let mut reason_code = ReasonCode::Success;
        let mut ack = ConnAck::new();
        ack.reason_code = reason_code;
        let mut prop = ConnAckProperties::new();
        prop.topic_alias_max = Some(300);
        ack.properties = Some(prop);
        let packet = Packet::ConnAck(ack);
        self.write_packet(packet).await?;

        Ok(true)
    }
}
