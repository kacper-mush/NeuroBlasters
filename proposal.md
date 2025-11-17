# Rust big project proposal
by Kacper Grzybowski, Marcin Rolbiecki, Mateusz Kasprzak

## NeuroBlasters
**A mutliplayer 2D top-down shooter where the AI evolves each round.**

![Example image](mockup-image.png)

### Expected features:
- a multiplayer game in the server-client model with a concurrent server handling multiple games at once,
- 2D, top-down scenery with a few simple maps to choose (like one shown above),
- players can move around, rotate the gun, and shoot projectiles,
- main game mode is a team deathmatch with a fixed-size team and remaining slots filled by AI,
- player can train their own AI in a private session, watching each epoch become smarter and smarter, and with the option to tweak certain ML parameters, and then use it against other players in a match (bring-your-own-AI),
- additionally players can choose to use a supplied AI model (or to just play only vs other players)
- a UI to navigate all those features.

### Technologies used:
- Tokio
- ... ? TODO


### First iteration:
Full game with only a mockup of an AI (moves around randomly)
### Second iteration:
Fully functional AI with previously described features, improved UI
### Additional:
Grenade throwing, more game modes, SFX and VFX

