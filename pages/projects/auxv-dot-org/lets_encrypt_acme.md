<head>
  <title>Let's Encrypt Acme | Auxv.org</title>
  <meta name="author" content="Owen Friedman">
  <meta name="description" content="How to implement Let's Encrypt certification with the Rocket WebFramwork and Rust 🗳️🚀⚙️...">
</head>

# TLS via Let's Encrypt + Rocket + Rust 🗳️🚀⚙️

After building this entire dang website with `axum`, I had the sudden realization that I wasn't particularly fond of using it. Like any self-respecting developer, I decided to stay up "late" (like 9PM) and port everything to `Rocket`.

<br/>

The migration was surprisingly smooth sailing(⛵) that is until I started setting up `https`. The `axum` server used `rustls-acme` and **Let's Encrypt** to automatically renew and verify my SSL certificates. 

<br/>
<details>
<summary><b>Table of Contents:</b></summary>

- [TLS via Let's Encrypt + Rocket + Rust 🗳️🚀⚙️](#tls-via-lets-encrypt--rocket--rust)
  - [How Let's Encrypt Works 🔐⬆️⬇️](#how-lets-encrypt-works)
  - [The Discoveries 🧪](#the-discoveries)
  - [Copy and Past(e | a) 🍝](#copy-and-paste--a)

</details>


## How Let's Encrypt Works 🔐⬆️⬇️

Let's Encrypt is a non-profit certificate authority run by the Internet Security Research Group. They provide _**freeeeeee**_ certificates to anyone who can prove they control a domain. This works by presenting the server with a challenge; if the server succeeds, it is given a key that can be used to create, renew, and revoke certificates for that domain.

<br/>

The `rustls-acme` crate handles this process automatically using the [TLS-ALPN-01](https://letsencrypt.org/docs/challenge-types/#tls-alpn-01) challenge, which occurs during the TLS handshake on the same port as your `https` traffic. You can set it up with `axum` like so:

```rs
use rustls_acme::{caches::DirCache, AcmeConfig};
use tokio_stream::StreamExt;

let acceptor = {
    let mut state = AcmeConfig::new(vec!["auxv.org"])
        .contact(vec!["mailto:5-pebble@protonmail.com"])
        .cache_option(Some(DirCache::new("lets_encrypt_cache")))
        .directory_lets_encrypt(true)
        .state();

    let tmp = state.axum_acceptor(state.default_rustls_config());
    tokio::spawn(async move {
        loop {
            match state.next().await.unwrap() {
                Ok(ok) => println!("Acme Event: {:?}", ok),
                Err(err) => println!("Acme Error: {:?}", err),
            }
        }
    });

    tmp
};

axum_server::bind(address)
    .acceptor(acceptor)
    .serve(app.into_make_service())
    .await?;
```

And that's it; you have `https` all setup.

<br/>

but...

<br/>

There is no support for `Rocket` nor has there been for the [last 8 years](https://github.com/rwf2/Rocket/issues/349). Thus, I found myself digging through the `Rocket` source code for the second time this week.


## The Discoveries 🧪

I found three undocumented and unreleased traits in addition to two methods.

> **Important Note:** As of January 2025 (`Rocket` 0.5.1), these features aren't part of the stable release. You'll need to import `Rocket` directly from the repository:
>
> ```toml
> # Cargo.toml
> [dependencies]
> rocket = { git = "https://github.com/rwf2/Rocket.git", features = ["tls"] }
> ```


**Traits:**
- `Bind` Configures listener startup behavior
- `Listener`: Handles incoming connections
- `Connection`: Represents an established connection


```rs
pub trait Bind: Listener + 'static {
    type Error: Error + Send + 'static;

    #[crate::async_bound(Send)]
    async fn bind(rocket: &Rocket<Ignite>) -> Result<Self, Self::Error>;

    fn bind_endpoint(to: &Rocket<Ignite>) -> Result<Endpoint, Self::Error>;
}

pub trait Listener: Sized + Send + Sync {
    type Accept: Send;

    type Connection: Connection;

    #[crate::async_bound(Send)]
    async fn accept(&self) -> io::Result<Self::Accept>;

    #[crate::async_bound(Send)]
    async fn connect(&self, accept: Self::Accept) -> io::Result<Self::Connection>;

    fn endpoint(&self) -> io::Result<Endpoint>;
}

pub trait Connection: AsyncRead + AsyncWrite + Send + Unpin {
    fn endpoint(&self) -> io::Result<Endpoint>;

    /// DER-encoded X.509 certificate chain presented by the client, if any.
    ///
    /// The certificate order must be as it appears in the TLS protocol: the
    /// first certificate relates to the peer, the second certifies the first,
    /// the third certifies the second, and so on.
    ///
    /// Defaults to an empty vector to indicate that no certificates were
    /// presented.
    fn certificates(&self) -> Option<Certificates<'_>> { None }
}
```
> Yes, `Connection` is half documented, but it's the half we aren't using. 😭

**Methods:**

- `launch_with`: Starts the server on the listener provided by the `Bind` trait
- `lanuch_on`: Accepts the configured listener as an argument


```rs
pub async fn launch_with<B: Bind>(self) -> Result<Rocket<Ignite>, Error> {
    let rocket = self.into_ignite().await?;
    let bind_endpoint = B::bind_endpoint(&rocket).ok();
    let listener: B = B::bind(&rocket).await
        .map_err(|e| ErrorKind::Bind(bind_endpoint, Box::new(e)))?;
    let any: Box<dyn Any + Send + Sync> = Box::new(listener);
    match any.downcast::<DefaultListener>() {
        Ok(listener) => {
            let listener = *listener;
            crate::util::for_both!(listener, listener => {
                crate::util::for_both!(listener, listener => {
                    rocket._launch(listener).await
                })
            })
        }
        Err(any) => {
            let listener = *any.downcast::<B>().unwrap();
            rocket._launch(listener).await
        }
    }
}

pub async fn launch_on<L>(self, listener: L) -> Result<Rocket<Ignite>, Error>
    where L: Listener + 'static,
{
    self.into_ignite().await?._launch(listener).await
}
```

<br/>

Here's how (I think) these traits operate:

- `Bind::bind`: Initializes and returns a new listener
- `Bind::bind_endpoint`: The local address the listener will be bound to

- `Listener::accept`: Discovers new connections; it needs to be fast as it's blocking
- `Listener::connect`: Handles connection initialization without blocking
- `Listener::endpoint`: The local address of the listener

- `Connection::endpoint`: The local half of the connection address
- `Connection::certificates`: I assume this returns the client's certificates for `mTLS`?

<br/>

With that, we can write a simple `LetsEncryptListener` to automate certificate creation and renewal.

## Copy and Past(e | a) 🍝

I will update this code as needed and plan to submit a **PR** to `rustls-acme` once these features stabilize in `Rocket`: 👋

```rs
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
```

> The End... 🌈🍯🍀
