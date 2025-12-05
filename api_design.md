## Messages

### Player → Server
`CONNECT <nickname>` – initiate session (optionally include client version)  
`DISCONNECT` – request graceful shutdown  
`ROOM CREATE` – create a lobby  
`ROOM JOIN <room_code>` – enter an existing lobby  
`ROOM LEAVE` – exit the lobby/matchmaking queue  
`INPUT <tick_id> <payload>` – send compressed input for the referenced simulation tick  

### Server → Player
`CONNECT OK` / `CONNECT ERROR <error>` – outcome of the handshake  
`SERVER ERROR <code> <message>` – catch-all fatal error (client should drop to menu)  

`ROOM CREATE OK <room_code>` / `ROOM CREATE ERROR <error>` – lobby creation result
`ROOM JOIN OK <room_state_id> <payload>` / `ROOM JOIN ERROR <error>` – room join result, full lobby state (players, settings) or error
`ROOM STATE <payload>` – lobby state broadcast whenever player list / settings change
`ROOM LEAVE OK` / `ROOM LEAVE ERROR` - room leave result; players can't leave the room once game started.

`GAME START <game_id>` / `GAME END <game_id>` – match lifecycle  
`ROUND START <round_id>` / `ROUND END <round_id>` – round lifecycle  
`GAME MAP <payload>` - all static objects loaded before the game starts
`GAME STATE <game_id> <tick_id> <payload>` – authoritative full game state - all dynamic objects and events; sent on every tick
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
