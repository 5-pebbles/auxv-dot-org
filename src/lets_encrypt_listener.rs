pub struct LetsEncryptListener(
    tokio::sync::Mutex<
        rustls_acme::tokio::TokioIncoming<
            tokio_util::compat::Compat<tokio::net::TcpStream>,
            std::io::Error,
            rustls_acme::tokio::TokioIncomingTcpWrapper<
                tokio::net::TcpStream,
                std::io::Error,
                tokio_stream::wrappers::TcpListenerStream,
            >,
            std::io::Error,
            std::io::Error,
        >,
    >,
);

impl LetsEncryptListener {
    pub async fn new() -> Self {
        let tcp_listener = tokio::net::TcpListener::bind((std::net::Ipv6Addr::UNSPECIFIED, 443))
            .await
            .unwrap();
        let tcp_incoming = tokio_stream::wrappers::TcpListenerStream::new(tcp_listener);
        let incoming = tokio::sync::Mutex::new(
            rustls_acme::AcmeConfig::new(vec!["auxv.org"])
                .contact(vec!["mailto:5-pebble@protonmail.com"])
                .cache_option(Some(rustls_acme::caches::DirCache::new(
                    "lets_encrypt_cache",
                )))
                .directory_lets_encrypt(true)
                .tokio_incoming(tcp_incoming, Vec::new()),
        );

        Self(incoming)
    }
}

impl rocket::listener::Listener for LetsEncryptListener {
    type Accept = LetsEncryptConnection;

    type Connection = Self::Accept;

    async fn accept(&self) -> std::io::Result<Self::Accept> {
        self.0
            .lock()
            .await
            .next()
            .await
            .unwrap()
            .map(LetsEncryptConnection)
    }

    async fn connect(&self, accept: Self::Accept) -> std::io::Result<Self::Connection> {
        Ok(accept)
    }

    fn endpoint(&self) -> std::io::Result<rocket::listener::Endpoint> {
        Ok(rocket::listener::Endpoint::Tcp(std::net::SocketAddr::from(
            (std::net::Ipv6Addr::UNSPECIFIED, 443),
        )))
    }
}

pub struct LetsEncryptConnection(
    tokio_util::compat::Compat<
        rustls_acme::futures_rustls::server::TlsStream<
            tokio_util::compat::Compat<tokio::net::TcpStream>,
        >,
    >,
);

impl tokio::io::AsyncWrite for LetsEncryptConnection {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        std::pin::Pin::new(&mut self.get_mut().0).poll_write(cx, buf)
    }
    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        std::pin::Pin::new(&mut self.get_mut().0).poll_flush(cx)
    }
    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        std::pin::Pin::new(&mut self.get_mut().0).poll_shutdown(cx)
    }
}

impl tokio::io::AsyncRead for LetsEncryptConnection {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        std::pin::Pin::new(&mut self.get_mut().0).poll_read(cx, buf)
    }
}

impl rocket::listener::Connection for LetsEncryptConnection {
    fn endpoint(&self) -> std::io::Result<rocket::listener::Endpoint> {
        Ok(rocket::listener::Endpoint::Tcp(std::net::SocketAddr::from(
            (std::net::Ipv6Addr::UNSPECIFIED, 443),
        )))
    }
}
