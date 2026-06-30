//! Prometheus metrics for the relay, exposed at `GET /metrics`.
//!
//! Phase-1 observability: live subscriber count, fan-out throughput, and — the
//! signal that matters for a single-instance broadcast relay — `lagged_total`
//! (slow subscribers dropping events) and the per-message send count.

use std::sync::OnceLock;

use prometheus::{Encoder, IntCounter, IntCounterVec, IntGauge, Opts, Registry, TextEncoder};

pub struct Metrics {
    pub registry: Registry,
    /// ConfirmedEvents accepted from nodes and fanned out.
    pub events_forwarded: IntCounter,
    /// Ingest rejections, by kind (decode | signature).
    pub ingest_errors: IntCounterVec,
    /// Currently connected WebSocket subscribers.
    pub subscribers: IntGauge,
    /// Messages successfully sent to subscribers.
    pub messages_sent: IntCounter,
    /// Times a subscriber lagged and dropped events (broadcast backlog).
    pub lagged: IntCounter,
}

static METRICS: OnceLock<Metrics> = OnceLock::new();

pub fn metrics() -> &'static Metrics {
    METRICS.get_or_init(|| {
        let registry = Registry::new();
        let events_forwarded = IntCounter::new(
            "earthnet_relay_events_forwarded_total",
            "ConfirmedEvents fanned out to subscribers",
        )
        .expect("metric");
        let ingest_errors = IntCounterVec::new(
            Opts::new(
                "earthnet_relay_ingest_errors_total",
                "Relay ingest rejections",
            ),
            &["kind"],
        )
        .expect("metric");
        let subscribers = IntGauge::new(
            "earthnet_relay_subscribers",
            "Currently connected WebSocket subscribers",
        )
        .expect("metric");
        let messages_sent = IntCounter::new(
            "earthnet_relay_messages_sent_total",
            "Messages successfully sent to subscribers",
        )
        .expect("metric");
        let lagged = IntCounter::new(
            "earthnet_relay_lagged_total",
            "Times a subscriber lagged and dropped events",
        )
        .expect("metric");

        registry
            .register(Box::new(events_forwarded.clone()))
            .expect("register");
        registry
            .register(Box::new(ingest_errors.clone()))
            .expect("register");
        registry
            .register(Box::new(subscribers.clone()))
            .expect("register");
        registry
            .register(Box::new(messages_sent.clone()))
            .expect("register");
        registry
            .register(Box::new(lagged.clone()))
            .expect("register");

        Metrics {
            registry,
            events_forwarded,
            ingest_errors,
            subscribers,
            messages_sent,
            lagged,
        }
    })
}

pub fn encode() -> String {
    let mut buf = Vec::new();
    let encoder = TextEncoder::new();
    let _ = encoder.encode(&metrics().registry.gather(), &mut buf);
    String::from_utf8(buf).unwrap_or_default()
}
