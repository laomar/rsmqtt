mod client;
mod hook;
mod link;
mod packet;
mod server;

pub use hook::*;
pub use link::*;
pub use packet::*;
pub use server::*;

use num_enum::TryFromPrimitiveError;
use tokio::io;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::error::Elapsed;
use tokio_rustls::rustls::pki_types::pem::Error as PemError;
use tokio_rustls::rustls::Error as RustlsError;

pub trait S: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T> S for T where T: AsyncRead + AsyncWrite + Unpin + Send {}

#[derive(Debug, thiserror::Error)]
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
    #[error("Invalid packet type: {0}")]
    TryFromPacketType(#[from] TryFromPrimitiveError<PacketType>),
}
