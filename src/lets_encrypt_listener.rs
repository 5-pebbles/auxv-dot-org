use std::{net::Ipv6Addr, sync::Arc};

use rocket::{listener::Listener, tls::TlsStream};
use rustls_acme::{
    AcmeConfig, caches::DirCache, futures_rustls::rustls::ServerConfig, is_tls_alpn_challenge,
};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
};
use tokio_rustls::LazyConfigAcceptor;
use tokio_stream::StreamExt;

pub struct LetsEncryptListener {
    listener: TcpListener,
    default_rustls_config: Arc<ServerConfig>,
    challenge_rustls_config: Arc<ServerConfig>,
}

impl LetsEncryptListener {
    pub async fn new() -> Self {
        // Enable TLS via Let's Encrypt:
        let mut state = AcmeConfig::new(vec!["auxv.org"])
            .contact(vec!["mailto:5-pebble@protonmail.com"])
            .cache_option(Some(DirCache::new("lets_encrypt_cache")))
            .directory_lets_encrypt(true)
            .state();
        let challenge_rustls_config = state.challenge_rustls_config();
        let default_rustls_config = state.default_rustls_config();

        tokio::spawn(async move {
            loop {
                match state.next().await.unwrap() {
                    Ok(ok) => log::info!("event: {:?}", ok),
                    Err(err) => log::error!("error: {:?}", err),
                }
            }
        });

        let listener = TcpListener::bind((Ipv6Addr::UNSPECIFIED, 443))
            .await
            .unwrap();

        Self {
            listener,
            default_rustls_config,
            challenge_rustls_config,
        }
    }
}

impl Listener for LetsEncryptListener {
    type Accept = TlsStream<TcpStream>;

    type Connection = Self::Accept;

    async fn accept(&self) -> std::io::Result<Self::Accept> {
        loop {
            let (tcp, _) = self.listener.accept().await.unwrap();
            let start_handshake = LazyConfigAcceptor::new(Default::default(), tcp)
                .await
                .unwrap();

            if is_tls_alpn_challenge(&start_handshake.client_hello()) {
                log::info!("received TLS-ALPN-01 validation request");
                let mut tls = start_handshake
                    .into_stream(self.challenge_rustls_config.clone())
                    .await
                    .unwrap();
                tls.shutdown().await.unwrap();
            } else {
                return start_handshake
                    .into_stream(self.default_rustls_config.clone())
                    .await;
            }
        }
    }

    async fn connect(&self, accept: Self::Accept) -> std::io::Result<Self::Connection> {
        Ok(accept)
    }

    fn endpoint(&self) -> std::io::Result<rocket::listener::Endpoint> {
        Ok(rocket::listener::Endpoint::Tcp(self.listener.local_addr()?))
    }
}
