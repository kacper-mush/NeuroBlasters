# Server Design

## Client State Machine

States: NotConnected, Connected, InGame(GameCode, PlayerID)

ClientEvents: ClientConnected, ClientDisconnected, ClientJoinedGame(GameCode), ClientLeftGame

Transitions:

NotConnected --(ClientConnected)--> Connected

Connected --(ClientDisconnected)--> Connected

Connected --(ClientJoinedGame(GameCode) if game is Waiting)--> InGame(GameCode, PlayerID)

InGame --(ClientLeftGame if game is Waiting)--> Disconnected

InGame --(ClientDisconnected)--> NotConnected


## Game State Machine

States: Waiting(players), BattleStarted(players, battle), BattleEnded(players)

GameEvents: PlayerJoined, PlayerLeft, PlayerDisconnected, PlayerInput, GameTick


| Message          | NotConnected | Connected | InGame(Waiting) | InGame(Countdown) | InGame(Started) | InGame(Ended) |
| ---------------- | :----------: | :-------: | :-------------: | :---------------: | :-------------: | :-----------: |
| `Connect`        |      ✅       |     ❌     |        ❌        |         ❌         |        ❌        |       ❌       |
| `Disconnect`     |      ❌       |     ✅     |        ✅        |         ✅         |        ✅        |       ✅       |
| `CreateGame`     |      ❌       |     ✅     |        ❌        |         ❌         |        ❌        |       ❌       |
| `JoinGame`       |      ❌       |     ✅     |        ❌        |         ❌         |        ❌        |       ❌       |
| `LeaveGame`      |      ❌       |     ❌     |        ✅        |         ✅         |        ❌        |       ✅       |
| `StartCountdown` |      ❌       |     ❌     |        ✅        |         ❌         |        ❌        |       ❌       |
| `PlayerInput`    |      ❌       |     ❌     |        ❌        |         ❌         |        ✅        |       ❌       |




Initial state:

--(Connect)--> Connected

From Connected:

Connected --(Disconnect)--> 

Connected --(CrateGame)--> InGame(Waiting)

Connected --(JoinGame(GameCode), GameCode is valid)--> InGame(Waiting / Countdown)

From Waiting:

Waiting --(JoinGame, players>=1)--> Waiting

Waiting --(LeaveGame, players>=1)--> Waiting

Waiting --(LeaveGame, players==0)--> Destroyed

Waiting --(StartCountdown, players>=2)--> Countdown

From Countdown:

Countdown --(PlayerJoin, players >= 2, remaining_ticks > 0)--> Countdown

Countdown --(PlayerLeave, players >= 2, remaining_ticks > 0)--> Countdown

Countdown --(PlayerLeave, players < 2)--> Waiting

Countdown --(remaining_ticks <= 0)--> Started

From Started:

Started --(PlayerInput)--> Started

Started --(Disconnect, players > 0)--> Started

Started --(blue team dead | red team dead)--> Ended

From Ended:

Ended --(PlayerLeave, players>=1)--> Ended

Ended --(PlayerLeave, players==0)--> Destroyed