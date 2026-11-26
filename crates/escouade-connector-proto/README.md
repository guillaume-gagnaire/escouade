# escouade-connector-proto

The protocol between `escouade-server` and `escouade-connector` (RP-2, ESC-598): the way a server outside a company
reaches the hosts of the company's network through a connector the company runs (`docs/PRIVATE-NETWORK.md` § 4, the
spec page « Escouade à distance › Atteindre un réseau privé », the decision `reseau-prive-rp-2-la-forme-du-protocole`).
Nothing of the core of Escouade nor of Tauri: `tokio`, `rustls`, `tokio-tungstenite`, `snow`.

| Module      | What                                                                                                                                   |
| ----------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `transport` | `wss://…/connector`: the connector's `connect` (an HTTP `CONNECT` proxy, Basic; a file of authorities), the server's TLS and upgrade |
| `noise`     | `Noise_IK_25519_ChaChaPoly_BLAKE2s` without I/O: the keys, the initiator (connector), the responder (server), the transport           |
| `version`   | The hello and the welcome; the server accepts its minor and the two before                                                             |
| `frame`     | `open`, `opened`, `refused`, `data` (empty: the end of a direction), `close`, `window`, `ping`, `pong`, `ceiling`                      |
| `channel`   | The handshake on a WebSocket, then the streams: a window per stream and direction, the ping, the cut, the bounds                       |
| `limits`    | The bounds (256 streams, 32 opens a second, 10 min of silence, the size of a frame) and the protocol's constants                     |
| `life`      | The connector's schedule of reconnection                                                                                               |

## Use

```rust
// The connector (RP-3).
let ws = transport::connect(&config).await?;
let mut channel = connect_server(ws, &keys, &server_key, version, &ceiling, limits).await?;
while let Some(incoming) = channel.next_open().await {
    // Check incoming.open() against the ceiling, resolve, connect, then:
    let stream = incoming.accept(); // or incoming.refuse(Refusal::new(RefusalCode::NotAllowed, "…"))
}

// The server's entry (RP-4).
let tls = acceptor.accept(tcp).await?;
let ws = transport::accept_websocket(tls, limits.handshake_timeout).await?;
match accept_connector(ws, &keys, |key| paired.contains(key), version, limits).await {
    Ok(channel) => { /* channel.open("gitlab.acme.internal", 22, agent, project).await */ }
    Err(e) if e.is_failed_attempt() => { /* count it: 20 a minute, § 4.5 */ }
    Err(_) => {}
}
```

## Tests

`cargo nextest run -p escouade-connector-proto` (from `src-tauri/`): the vectors (`tests/vectors.rs`), the channel over
an in-memory WebSocket in simulated time (`tests/channel.rs`), the real transport on 127.0.0.1 with an authority made in
the test, a fake company proxy and a fake inspection of TLS (`tests/transport.rs`). Keys are made in the tests; none is
written. The fuzzing targets `connector-frames` and `connector-handshake` are in `fuzz/` (`sh scripts/fuzz.sh 60
connector-frames connector-handshake`, Linux or macOS); their seeds are played by the crate's own tests
(`ESCOUADE_WRITE_SEEDS=1 cargo test -p escouade-connector-proto --lib fuzzing` writes them again).
