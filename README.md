# NeuroBlasters
Authors:
- Marcin Rolbiecki
- Mateusz Kasprzak
- Kacper Grzybowski

**See [proposal](proposal.md)**

## POC Server

The proof-of-concept server uses `renet`/`renet_netcode` for reliable channels and follows the message
contract described in `api_design.md`. A mocked game loop emits deterministic `GAME SNAPSHOT` and
`GAME DELTA` messages so that network, lobby, and serialization layers can already be tested end to end.


### Run the server

```bash
cargo run
```

The server starts on `127.0.0.1:5000` by default. Adjust binding/limits inside `server::ServerOptions`
if you need different ports or capacity.

### Run the mock client

```bash
cargo run --bin mock_client <nickname> [server_addr] [room_code]
```

- `nickname` – display name used during `CONNECT`
- `server_addr` – optional socket address (defaults to `127.0.0.1:5000`)
- `room_code` – optional room to join; if omitted the client issues `ROOM CREATE`

A single mock client handles the entire handshake sequence, sends fake inputs on the dedicated channel,
and logs snapshots/deltas from the server.

### Tests

```bash
cargo test
```

Protocol serialization/deserialization tests live in `src/protocol.rs` and ensure binary compatibility.
