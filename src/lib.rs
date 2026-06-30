//! earthnet-relay — low-latency fan-out of [`ConfirmedEvent`](earthnet_protocol::ConfirmedEvent)s.
//!
//! Nodes POST signed ConfirmedEvents to `/events`; the relay verifies and fans
//! them out to all clients connected on the `/subscribe` WebSocket. The hot path
//! is in-memory (a broadcast channel) so a confirmed event reaches subscribers
//! without touching disk.
//!
//! v0.1 is a single-relay fan-out. Inter-relay gossip (libp2p/gossipsub) is a
//! later hardening slice (DESIGN §5).

pub mod metrics;
pub mod server;

use tokio::sync::broadcast;

/// Shared relay state: the broadcast channel carrying encoded ConfirmedEvent bytes.
#[derive(Clone)]
pub struct RelayState {
    pub tx: broadcast::Sender<Vec<u8>>,
}

impl RelayState {
    /// `capacity` = how many events a slow subscriber may lag before dropping.
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity.max(1));
        Self { tx }
    }

    /// Current number of connected subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}
