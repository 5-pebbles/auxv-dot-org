use std::net::Ipv4Addr;

use auxv_dot_org::{analytics, build_rocket, pages};
use clap::Parser;
use lets_encrypt_listener::LetsEncryptListener;
use rocket::listener::tcp::TcpListener;
use rustls_acme::{AcmeConfig, caches::DirCache};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

mod lets_encrypt_listener;

/// My personal markdown based webserver (though you are welcome to use it).
#[derive(Parser)]
#[command(version, about, propagate_version = true)]
struct Args {
    /// HTTP port to listen on
    #[arg(long, default_value = "80")]
    http_port: u16,

    /// HTTPS port to listen on
    #[arg(long, default_value = "443")]
    https_port: u16,

    /// Domain name for HTTPS certificate (required for HTTPS)
    #[arg(long)]
    domain: Option<String>,

    /// Let's Encrypt contact email (required for HTTPS)
    #[arg(long)]
    email: Option<String>,

    /// Disable https (for testing without Let's Encrypt)
    #[arg(long)]
    http_only: bool,

    /// Directory to store Let's Encrypt cache
    #[arg(long, default_value = "lets_encrypt_cache")]
    lets_encrypt_cache: String,

    /// Password for analytics database encryption
    #[arg(long, env = "ANALYTICS_PASSWORD")]
    analytics_password: Option<String>,

    /// Path to the SQLite analytics database file
    #[arg(long, default_value = "analytics.db")]
    analytics_db: String,
}

#[rocket::main]
async fn main() {
    let args = Args::parse();

    pages::set_page_cache().unwrap();

    let analytics = args.analytics_password.map(|password| {
        let analytics = analytics::Analytics::open(&args.analytics_db, &password)
            .expect("Failed to open analytics database");
        analytics.cleanup_old_records();
        analytics
    });

    let rocket = build_rocket(analytics);

    let http_listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, args.http_port))
        .await
        .unwrap();

    if args.http_only {
        rocket.launch_on(http_listener).await.unwrap();
    } else {
        // HTTP Listener for redirection:
        tokio::spawn(async move {
            loop {
                if let Ok((mut socket, _)) = http_listener.accept().await {
                    tokio::spawn(async move {
                        let mut buf = [0; 1024];
                        if socket.read(&mut buf).await.is_ok() {
                            // Simple HTTP 301 redirect response:
                            let response = "HTTP/1.1 301 Moved Permanently\r\n\
                                          Location: https://auxv.org\r\n\
                                          Connection: close\r\n\r\n";
                            let _ = socket.write_all(response.as_bytes()).await;
                        }
                    });
                }
            }
        });

        // Validate required HTTPS parameters:
        let domain = args.domain.expect("Domain is required when using HTTPS");
        let email = args.email.expect("Email is required when using HTTPS");

        // Enable HTTPS via Let's Encrypt:
        let tcp_listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, args.https_port))
            .await
            .unwrap();
        let acme_config = AcmeConfig::new([domain])
            .contact([format!("mailto:{}", email)])
            .cache_option(Some(DirCache::new(args.lets_encrypt_cache)))
            .directory_lets_encrypt(true);

        let https_listener = LetsEncryptListener::new(acme_config, tcp_listener).await;
        rocket.launch_on(https_listener).await.unwrap();
    }
}
