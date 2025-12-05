## Messages

### Player → Server
`CONNECT <nickname>` – initiate session (optionally include client version)  
`DISCONNECT` – request graceful shutdown  
`ROOM CREATE` – create a lobby  
`ROOM JOIN <room_code>` – enter an existing lobby  
`ROOM LEAVE` – exit the lobby/matchmaking queue  
`INPUT <tick_id> <payload>` – send compressed input for the referenced simulation tick  
`RESYNC REQUEST <game_id>` – ask for a fresh snapshot if the client detects a gap or corruption

### Server → Player
`CONNECT OK` / `CONNECT ERROR <error>` – outcome of the handshake  
`SERVER ERROR <code> <message>` – catch-all fatal error (client should drop to menu)  

`ROOM CREATE OK <room_code>` / `ROOM CREATE ERROR <error>` – lobby creation result
`ROOM JOIN OK <room_state_id> <payload>` / `ROOM JOIN ERROR <error>` – room join result, full lobby state (players, settings) or error
`ROOM DELTA <room_state_id> <delta_payload>` – diff-only lobby update broadcast whenever player list / settings change
`ROOM LEAVE OK` / `ROOM LEAVE ERROR` - room leave result; players can't leave the room once game started.

`GAME START <game_id>` / `GAME END <game_id>` – match lifecycle  
`ROUND START <round_id>` / `ROUND END <round_id>` – round lifecycle  
`GAME SNAPSHOT <game_id> <tick_id> <payload>` – authoritative full game state; sent on game start, after resync, or on demand  
`GAME DELTA <game_id> <tick_id> <base_tick> <delta_payload>` – state diff relative to the referenced `base_tick` snapshot/delta  
`INPUT ERROR <tick_id> <error>` – rejects a specific input packet when it fails validation (e.g., throttling, illegal action)



## Map

Only circles and rectangles.

## Game state

Map (dimensions, set of shapes with positions, spawnpoints),
Dynamic objects[
    Players (position, health, rotation, inventory, team, is_ai),
    Projectiles (position, direction, speed),
    ...other objects...
]

`GAME SNAPSHOT` carries the full structure above, while `GAME DELTA` contains only the entries that changed plus tombstone lists (e.g., `players_removed`, `projectiles_removed`). Both payloads should include a `tick_id` and checksum so clients can discard stale data or trigger `RESYNC REQUEST`.