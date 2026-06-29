use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use earthnet_protocol::{sign, ConfirmedEvent, EvidenceKind, Location, PROTOCOL_VERSION};
use earthnet_relay::{server::app, RelayState};
use ed25519_dalek::SigningKey;
use futures_util::StreamExt;
use prost::Message;
use rand::{rngs::OsRng, RngCore};
use tokio::net::TcpListener;
use tower::ServiceExt;

fn signed_event() -> ConfirmedEvent {
    let mut secret = [0u8; 32];
    OsRng.fill_bytes(&mut secret);
    let key = SigningKey::from_bytes(&secret);
    let mut evt = ConfirmedEvent {
        protocol_version: PROTOCOL_VERSION,
        event_id: vec![7u8; 16],
        pubkey: key.verifying_key().to_bytes().to_vec(),
        origin_time_ns: 1_700_000_000_000_000_000,
        issued_at_ns: 1_700_000_000_300_000_000,
        epicenter: Some(Location {
            geohash: "66jd2k".into(),
            precision_m: 600,
        }),
        depth_km: 35.0,
        magnitude: 6.2,
        magnitude_uncert: 0.3,
        evidence: EvidenceKind::Official as i32,
        num_observations: 1,
        obs_ids: vec![vec![1u8; 16]],
        supersedes: Vec::new(),
        signature: Vec::new(),
    };
    sign(&key, &mut evt);
    evt
}

#[tokio::test]
async fn health_ok() {
    let resp = app(RelayState::new(16))
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn post_valid_event_accepted() {
    let resp = app(RelayState::new(16))
        .oneshot(
            Request::post("/events")
                .body(Body::from(signed_event().encode_to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
}

#[tokio::test]
async fn post_garbage_is_bad_request() {
    let resp = app(RelayState::new(16))
        .oneshot(
            Request::post("/events")
                .body(Body::from(vec![0xde, 0xad]))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn post_tampered_event_unauthorized() {
    let mut bytes = signed_event().encode_to_vec();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xff;
    let resp = app(RelayState::new(16))
        .oneshot(Request::post("/events").body(Body::from(bytes)).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn fan_out_delivers_event_to_subscriber() {
    let state = RelayState::new(16);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server_app = app(state.clone());
    tokio::spawn(async move {
        axum::serve(listener, server_app).await.unwrap();
    });

    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/subscribe"))
        .await
        .expect("ws connect");

    // wait until the subscriber is registered on the broadcast channel
    for _ in 0..50 {
        if state.subscriber_count() > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(state.subscriber_count() > 0, "subscriber not registered");

    let bytes = signed_event().encode_to_vec();
    state.tx.send(bytes.clone()).unwrap();

    let msg = tokio::time::timeout(Duration::from_secs(2), ws.next())
        .await
        .expect("timed out")
        .expect("stream ended")
        .expect("ws error");
    assert!(msg.is_binary());
    assert_eq!(msg.into_data().to_vec(), bytes);
}
