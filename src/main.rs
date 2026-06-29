//! earthnet-relay entrypoint.

use earthnet_relay::{server::app, RelayState};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "earthnet_relay=info".into()),
        )
        .init();

    let capacity: usize = std::env::var("EARTHNET_RELAY_CAPACITY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(256);
    let state = RelayState::new(capacity);

    let addr = std::env::var("EARTHNET_RELAY_ADDR").unwrap_or_else(|_| "127.0.0.1:8090".into());
    let listener = TcpListener::bind(&addr).await.expect("bind address");
    tracing::info!(%addr, capacity, "earthnet-relay listening");

    axum::serve(listener, app(state))
        .await
        .expect("server error");
}
