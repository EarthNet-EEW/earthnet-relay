//! HTTP + WebSocket surface for the relay.

use axum::{
    body::Bytes,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use earthnet_protocol::{verify, ConfirmedEvent};
use prost::Message as _;
use tokio::sync::broadcast::error::RecvError;

use crate::metrics::metrics;
use crate::RelayState;

/// Builds the relay router.
pub fn app(state: RelayState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics_handler))
        .route("/events", post(ingest))
        .route("/subscribe", get(subscribe))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

/// Prometheus metrics in text exposition format.
async fn metrics_handler() -> impl IntoResponse {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4",
        )],
        crate::metrics::encode(),
    )
}

/// Node → relay. Verifies a ConfirmedEvent and fans the raw bytes out.
///   202 Accepted     verified + fanned out
///   400 Bad Request  undecodable
///   401 Unauthorized signature failed
async fn ingest(State(state): State<RelayState>, body: Bytes) -> StatusCode {
    let evt = match ConfirmedEvent::decode(body.as_ref()) {
        Ok(e) => e,
        Err(_) => {
            metrics().ingest_errors.with_label_values(&["decode"]).inc();
            return StatusCode::BAD_REQUEST;
        }
    };
    if verify(&evt).is_err() {
        metrics()
            .ingest_errors
            .with_label_values(&["signature"])
            .inc();
        return StatusCode::UNAUTHORIZED;
    }
    // Ignore send error: it only means there are no subscribers right now.
    let _ = state.tx.send(body.to_vec());
    metrics().events_forwarded.inc();
    tracing::info!(
        subscribers = state.subscriber_count(),
        "fanned out ConfirmedEvent"
    );
    StatusCode::ACCEPTED
}

/// Client → relay. Upgrades to a WebSocket and streams ConfirmedEvent bytes.
async fn subscribe(State(state): State<RelayState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| pump(socket, state))
}

async fn pump(mut socket: WebSocket, state: RelayState) {
    let mut rx = state.tx.subscribe();
    metrics().subscribers.inc();
    loop {
        tokio::select! {
            event = rx.recv() => match event {
                Ok(bytes) => {
                    if socket.send(Message::Binary(bytes)).await.is_err() {
                        break; // client gone
                    }
                    metrics().messages_sent.inc();
                }
                Err(RecvError::Lagged(_)) => {
                    metrics().lagged.inc(); // skip dropped events, keep latest
                    continue;
                }
                Err(RecvError::Closed) => break,
            },
            incoming = socket.recv() => match incoming {
                Some(Ok(_)) => {} // ignore client messages (pings handled by axum)
                _ => break,       // close / error
            },
        }
    }
    metrics().subscribers.dec();
}
