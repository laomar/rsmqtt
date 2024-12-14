use crate::*;
use async_tungstenite::tokio::accept_hdr_async;
use async_tungstenite::tungstenite::handshake::server::{
    Callback, ErrorResponse, Request, Response,
};
use async_tungstenite::tungstenite::http::HeaderValue;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::task;
use tokio_rustls::rustls::pki_types::pem::PemObject;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;
use ws_stream_tungstenite::WsStream;

pub struct MqttServer {
    listeners: Vec<Listener>,
    hook: Arc<Hook>,
    proxy_protocol: bool,
}
impl MqttServer {
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
            hook: Arc::new(Hook::new()),
            proxy_protocol: false,
        }
    }

    pub fn tcp(&mut self, addr: &str) -> &mut Self {
        self._listen("tcp", addr, "", "");
        self
    }
    pub fn tls(&mut self, addr: &str, cert: &str, key: &str) -> &mut Self {
        self._listen("tls", addr, cert, key);
        self
    }
    pub fn ws(&mut self, addr: &str) -> &mut Self {
        self._listen("ws", addr, "", "");
        self
    }
    pub fn wss(&mut self, addr: &str, cert: &str, key: &str) -> &mut Self {
        self._listen("wss", addr, cert, key);
        self
    }
    fn _listen(&mut self, protocol: &str, addr: &str, cert: &str, key: &str) -> &mut Self {
        self.listeners.push(Listener {
            protocol: protocol.to_owned(),
            addr: addr.to_owned(),
            cert: cert.to_owned(),
            key: key.to_owned(),
        });
        self
    }

    pub fn proxy_protocol(&mut self, proxy: bool) -> &mut Self {
        self.proxy_protocol = proxy;
        self
    }
    pub fn connect(
        &mut self,
        f: impl Fn(Connect) -> Result<Packet, Error> + Send + Sync + 'static,
    ) -> &mut Self {
        self.hook.register(move |packet| match packet {
            Packet::Connect(connect) => f(connect),
            _ => Ok(Packet::None),
        });
        self
    }
    pub fn publish(
        &mut self,
        f: impl Fn(Publish) -> Result<Packet, Error> + Send + Sync + 'static,
    ) -> &mut Self {
        self.hook.register(move |packet| match packet {
            Packet::Publish(publish) => f(publish),
            _ => Ok(Packet::None),
        });
        self
    }
    pub async fn run(&mut self) -> Result<(), Error> {
        if self.listeners.is_empty() {
            self.tcp("0.0.0.0:1883");
        }
        for listen in self.listeners.clone() {
            let hook = Arc::clone(&self.hook);
            task::spawn(async move {
                if let Err(e) = listen.start(hook).await {
                    println!("{}", e);
                }
            });
        }
        Ok(())
    }
}
#[derive(Debug, Clone)]
struct Listener {
    protocol: String,
    addr: String,
    cert: String,
    key: String,
}
impl Listener {
    async fn start(&self, hook: Arc<Hook>) -> Result<(), Error> {
        let protocol = self.protocol.as_str();
        println!("{}", protocol);
        let acceptor = match protocol {
            "tls" | "wss" => Some(self.acceptor()?),
            _ => None,
        };
        let listener = TcpListener::bind(&self.addr).await?;
        loop {
            let stream = match listener.accept().await {
                Ok((stream, addr)) => {
                    println!("{:?}", addr);
                    stream
                }
                Err(_) => continue,
            };
            let hook = Arc::clone(&hook);
            match protocol {
                "tcp" => {
                    let stream = Box::new(stream);
                    task::spawn(Link::new(stream, hook).serve());
                }
                "tls" => {
                    let acceptor = acceptor.clone().unwrap();
                    let stream = match acceptor.accept(stream).await {
                        Ok(stream) => Box::new(stream),
                        Err(_) => continue,
                    };
                    task::spawn(Link::new(stream, hook).serve());
                }
                "ws" => {
                    let stream = match accept_hdr_async(stream, WSCallback).await {
                        Ok(stream) => stream,
                        Err(_) => continue,
                    };
                    let stream = Box::new(WsStream::new(stream));
                    task::spawn(Link::new(stream, hook).serve());
                }
                "wss" => {
                    let acceptor = acceptor.clone().unwrap();
                    let stream = match acceptor.accept(stream).await {
                        Ok(stream) => stream,
                        Err(_) => continue,
                    };
                    let stream = match accept_hdr_async(stream, WSCallback).await {
                        Ok(stream) => stream,
                        Err(_) => continue,
                    };
                    let stream = Box::new(WsStream::new(stream));
                    task::spawn(Link::new(stream, hook).serve());
                }
                _ => (),
            }
        }
    }
    fn acceptor(&self) -> Result<TlsAcceptor, Error> {
        let key = PrivateKeyDer::from_pem_file(&self.key)?;
        let certs = CertificateDer::pem_file_iter(&self.cert)?.collect::<Result<Vec<_>, _>>()?;
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
