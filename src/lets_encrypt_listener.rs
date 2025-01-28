use std::{
    fmt::Debug,
    io::{Error, Result},
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};

use rocket::listener::{Connection, Endpoint, Listener};
use rustls_acme::{
    AcmeConfig,
    futures_rustls::server::TlsStream,
    tokio::{TokioIncoming, TokioIncomingTcpWrapper},
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::{TcpListener, TcpStream},
    sync::Mutex,
};
use tokio_stream::{StreamExt, wrappers::TcpListenerStream};
use tokio_util::compat::Compat;

/// 🔐 A Rocket-compatible HTTPS listener that handles Let's Encrypt certificate automation.
///
/// This listener wraps a TcpListener and manages automatic TLS certificate provisioning
/// and renewal through Let's Encrypt's ACME protocol. It implements Rocket's `Listener`
/// trait to provide HTTPS connections while simultaneously completing TLS-ALPN-01 challenges.
///
/// # Example
/// ```rust
/// let tcp_listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, 443)).await?;
/// let acme_config = AcmeConfig::new(["example.com"])
///     .contact(["mailto:admin@example.com"])
///     .directory_lets_encrypt(true);
///
/// let https_listener = LetsEncryptListener::new(acme_config, tcp_listener).await;
/// rocket.launch_on(https_listener).await?;
/// ```
pub struct LetsEncryptListener<T: Debug + 'static>(
    Mutex<
        TokioIncoming<
            Compat<TcpStream>,
            Error,
            TokioIncomingTcpWrapper<TcpStream, Error, TcpListenerStream>,
            T,
            T,
        >,
    >,
    SocketAddr,
);

impl<T: Debug + 'static> LetsEncryptListener<T> {
    /// Makes a new `LetEncryptListener` from the given ACME configuration and TCP listener.
    pub async fn new(acme_config: AcmeConfig<T, T>, tcp_listener: TcpListener) -> Self {
        let local_address = tcp_listener.local_addr().unwrap();
        let tcp_listener_stream = TcpListenerStream::new(tcp_listener);

        Self(
            Mutex::new(acme_config.tokio_incoming(tcp_listener_stream, Vec::new())),
            local_address,
        )
    }
}

impl<T: Debug + 'static> Listener for LetsEncryptListener<T> {
    type Accept = LetsEncryptConnection;

    type Connection = Self::Accept;

    async fn accept(&self) -> Result<Self::Accept> {
        self.0
            .lock()
            .await
            .next()
            .await
            .unwrap()
            .map(|tls_stream| LetsEncryptConnection(tls_stream, self.1))
    }

    async fn connect(&self, accept: Self::Accept) -> Result<Self::Connection> {
        Ok(accept)
    }

    fn endpoint(&self) -> Result<Endpoint> {
        Ok(Endpoint::Tcp(self.1))
    }
}

/// 🔐⬆️⬇️ A connection established through the Let's Encrypt listener.
pub struct LetsEncryptConnection(Compat<TlsStream<Compat<TcpStream>>>, SocketAddr);

impl AsyncWrite for LetsEncryptConnection {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        Pin::new(&mut self.get_mut().0).poll_write(cx, buf)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        std::pin::Pin::new(&mut self.get_mut().0).poll_flush(cx)
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.get_mut().0).poll_shutdown(cx)
    }
}

impl AsyncRead for LetsEncryptConnection {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        Pin::new(&mut self.get_mut().0).poll_read(cx, buf)
    }
}

impl Connection for LetsEncryptConnection {
    fn endpoint(&self) -> Result<Endpoint> {
        Ok(Endpoint::Tcp(self.1))
    }
}
