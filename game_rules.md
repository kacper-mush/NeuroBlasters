# Room Lifecycle

- Rooms are created on demand via `RoomCreate` and identified by a random six-character alphanumeric code that remains valid while the room is occupied. Empty rooms are retained for up to five minutes so players can rejoin briefly after disconnects, then they are purged.
- Each room tracks the list of authenticated sessions and broadcasts a `RoomUpdate` to every member whenever membership changes, countdown events occur, or other room-level events are queued.
- The server keeps a per-room event queue; every simulation tick drains pending events into the `RoomUpdate` payload so clients always see the latest state plus a concise history of what just happened (join/leave/countdown tick, etc.).

## Countdown Flow

1. Any room member can trigger `RoomStartCountdown { seconds }` once they are ready to start the match.
2. The server records the countdown length, emits `CountdownStarted`, and begins emitting `CountdownTick` events as full seconds elapse.
3. When the timer reaches zero a `CountdownFinished` event is pushed. The server currently stops the countdown there; future iterations can hook that event to launch gameplay.

## Error Handling Contracts

- **Connect**: API version must match `API_VERSION`, otherwise the server responds with `ServerError::Connect` and disconnects the transport.
- **RoomCreate / RoomJoin / RoomLeave**: errors are reported via the specific room-related variants (e.g. `ServerError::RoomJoin`) so clients can map them to UI messaging.
- **Countdown**: invalid requests (e.g. zero seconds or sending while not in a room) result in `ServerError::General`.

## Testing

- `room.rs` contains unit tests that verify membership bookkeeping and countdown emissions.
- `tests/room_flow.rs` spins up the real server binary, performs an end-to-end handshake using `RenetClient`, and asserts that `RoomCreate` and subsequent `RoomUpdate` broadcasts succeed.

