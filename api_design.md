## Messages

### Player
`CONNECT <nickname>`

`ROOM CREATE` 
`ROOM JOIN <room_code>`

`INPUT <input>`

### Server

`SERVER ERROR <error>`

`CONNECT OK`
`CONNECT ERROR`

`ROOM CREATE OK <room_code>`
`ROOM CREATE ERROR <error>`

`ROOM JOIN ERROR <error>`
`ROOM STATE <room state>`

`GAME START`
`GAME END`
`ROUND START`
`ROUND END`

`GAME STATE <game_state>`

`INPUT ERROR`



## Map

Only circles and rectangles.


## Game state

Map (dimensions, set of shapes with positions, spawnpoints),
Dynamic objects[
    Players (position, health, rotation, ...),
    Projectiles (position, direction),
    ...other objects...
]
