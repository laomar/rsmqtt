use async_tungstenite::tokio::{accept_async, accept_hdr_async};
use async_tungstenite::tungstenite::handshake::server::{
    Callback, ErrorResponse, Request, Response,
};
use async_tungstenite::tungstenite::http::HeaderValue;
use futures_util::StreamExt;
use std::error::Error;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tokio::net::TcpListener;
use tokio::task;
use tokio_rustls::rustls::pki_types::pem::PemObject;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;
use ws_stream_tungstenite::WsStream;

#[derive(Debug, Default)]
struct Listen {
    addr: String,
    path: String,
    cert: String,
    key: String,
}
#[derive(Debug, Default)]
pub struct Mqtt {
    listens: Vec<(String, Listen)>,
    max_connections: usize,
    proxy_protocol: bool,
}
impl Mqtt {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn tcp(&mut self, addr: &str) -> &mut Self {
        self.listens.push((
            "tcp".to_owned(),
            Listen {
                addr: addr.to_owned(),
                ..Default::default()
            },
        ));
        self
    }

    pub fn tls(&mut self, addr: &str, cert: &str, key: &str) -> &mut Self {
        self.listens.push((
            "tls".to_owned(),
            Listen {
                addr: addr.to_owned(),
                cert: cert.to_owned(),
                key: key.to_owned(),
                ..Default::default()
            },
        ));
        self
    }

    pub fn ws(&mut self, addr: &str, path: &str) -> &mut Self {
        self.listens.push((
            "ws".to_owned(),
            Listen {
                addr: addr.to_owned(),
                path: path.to_owned(),
                ..Default::default()
            },
        ));
        self
    }

    pub fn wss(&mut self, addr: &str, path: &str, cert: &str, key: &str) -> &mut Self {
        self.listens.push((
            "wss".to_owned(),
            Listen {
                addr: addr.to_owned(),
                path: path.to_owned(),
                cert: cert.to_owned(),
                key: key.to_owned(),
                ..Default::default()
            },
        ));
        self
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        if self.listens.is_empty() {
            self.tcp("0.0.0.0:1883");
        }

        for (protocol, listen) in &self.listens {
            println!("Listening on {}", protocol);

            let tls = match protocol.as_str() {
                "tls" | "wss" => {
                    let key = PrivateKeyDer::from_pem_file(&listen.key)?;
                    let certs = CertificateDer::pem_file_iter(&listen.cert)?
                        .collect::<Result<Vec<_>, _>>()?;
                    let config = ServerConfig::builder()
                        .with_no_client_auth()
                        .with_single_cert(certs, key)?;
                    Some(TlsAcceptor::from(Arc::new(config)))
                }
                _ => None,
            };

            let listener = TcpListener::bind(listen.addr.as_str()).await?;
            loop {
                let stream = match listener.accept().await {
                    Ok((s, addr)) => {
                        println!("{:?}", addr);
                        s
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                        continue;
                    }
                };

                match protocol.as_str() {
                    "tcp" => {
                        task::spawn(serve(stream));
                    }
                    "tls" => {
                        let stream = tls.clone().unwrap().accept(stream).await.unwrap();
                        task::spawn(serve(stream));
                    }
                    "ws" => {
                        let stream = accept_hdr_async(stream, WSCallback).await.unwrap();
                        let stream = WsStream::new(stream);
                        task::spawn(serve(stream));
                    }
                    "wss" => {
                        let stream = tls.clone().unwrap().accept(stream).await.unwrap();
                        let stream = accept_hdr_async(stream, WSCallback).await.unwrap();
                        let stream = WsStream::new(stream);
                        task::spawn(serve(stream));
                    }
                    _ => (),
                }
            }
        }

        Ok(())
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
async fn serve<T>(mut stream: T)
where
    T: S,
{
    let mut buf = vec![0; 1024];
    loop {
        match stream.read(&mut buf).await {
            Ok(0) => {
                println!("Connection closed");
                break;
            }
            Ok(n) => {
                println!("{:?}", &buf[0..n]);
            }
            Err(e) => {
                println!("read Error: {}", e);
                break;
            }
        }
    }
}
