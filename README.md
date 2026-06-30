> 🌎 Part of **[EarthNet](https://github.com/devjamez/earthnet)** — open-source, decentralized earthquake early warning for Latin America.

# earthnet-relay

Low-latency fan-out for [EarthNet](https://github.com/devjamez/earthnet-protocol).
Nodes POST signed `ConfirmedEvent`s; the relay verifies and pushes them to all
connected clients over a persistent WebSocket â€” the transport an Android
foreground service holds open to receive alerts with the screen off.

## API

```
POST /events       body = ConfirmedEvent protobuf
  202 Accepted     verified + fanned out
  400 Bad Request  undecodable
  401 Unauthorized signature failed
GET  /subscribe    WebSocket â†’ binary ConfirmedEvent frames
GET  /health â†’ "ok"
```

The hot path is an in-memory broadcast channel â€” a confirmed event reaches
subscribers without touching disk.

## Run

```sh
cargo run
# env: EARTHNET_RELAY_ADDR (default 127.0.0.1:8090), EARTHNET_RELAY_CAPACITY (default 256)
```

## Status

ðŸŸ¡ v0.1 â€” single-relay fan-out. Inter-relay gossip (libp2p/gossipsub, QUIC) is a
later hardening slice.

## License

AGPL-3.0-or-later.
