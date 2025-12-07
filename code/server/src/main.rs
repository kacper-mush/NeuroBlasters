mod client;
mod game;
mod game_manager;

use std::{collections::HashMap, net::SocketAddr, net::UdpSocket, time::Duration, time::Instant};

use client::{ClientEvent, ClientState};
use common::codec::{decode_client_message, encode_server_message};
use common::protocol::{
    ClientMessage, GameCode, GameUpdate, ServerMessage,
};
use game::{Game, GameCommand, GameState}; // Import our Game FSM
use game_manager::GameManager;


use renet::{ClientId, ConnectionConfig, RenetServer, ServerEvent};
use renet_netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig};
use tokio::time::{self, MissedTickBehavior};
use tracing::{debug, error, info, trace};
use tracing_subscriber::EnvFilter;

const SERVER_PORT: u16 = 8080;
const MAX_CLIENTS: usize = 64;
const PROTOCOL_ID: u64 = 0;
const RELIABLE_CHANNEL_ID: u8 = 0;
const TICK_INTERVAL: Duration = Duration::from_micros(33_333); // ≈30 Hz

type AppResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

// --- SERVER APP ---

#[tokio::main]
async fn main() -> AppResult<()> {
    init_tracing();

    let mut app = ServerApp::new()?;
    info!(
        addresses = ?app.public_addresses(),
        max_clients = MAX_CLIENTS,
        "listening for clients"
    );

    let mut ticker = time::interval(TICK_INTERVAL);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("received Ctrl+C, shutting down");
                break;
            }
            _ = ticker.tick() => {
                if let Err(err) = app.tick() {
                    error!(error = %err, "server tick failed");
                }
            }
        }
    }

    app.shutdown();
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}

struct ServerApp {
    server: RenetServer,
    transport: NetcodeServerTransport,
    
    // State
    client_states: HashMap<ClientId, ClientState>,
    game_manager: GameManager,
    
    last_tick: Instant,
}

impl ServerApp {
    fn new() -> AppResult<Self> {
        let current_time = Duration::ZERO;
        let public_addr: SocketAddr = ([0, 0, 0, 0], SERVER_PORT).into();
        let server_config = ServerConfig {
            current_time,
            max_clients: MAX_CLIENTS,
            protocol_id: PROTOCOL_ID,
            public_addresses: vec![public_addr],
            authentication: ServerAuthentication::Unsecure,
        };

        let socket = UdpSocket::bind(public_addr)?;
        let transport = NetcodeServerTransport::new(server_config, socket)?;
        let server = RenetServer::new(ConnectionConfig::default());

        Ok(Self {
            server,
            transport,
            client_states: HashMap::new(),
            game_manager: GameManager::new(),
            last_tick: Instant::now(),
        })
    }

    fn public_addresses(&self) -> Vec<SocketAddr> {
        self.transport.addresses()
    }

    fn tick(&mut self) -> AppResult<()> {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f32();
        self.last_tick = now;

        // 1. NETWORK: Receive packets
        self.transport.update(Duration::from_secs_f32(dt), &mut self.server)?;
        self.server.update(Duration::from_secs_f32(dt));

        // 2. LOGIC: Process connections and messages
        self.process_net_events();
        self.process_client_messages();

        // 3. GAMEPLAY: Tick all active games
        self.tick_games(dt);

        // 4. NETWORK: Send packets
        self.transport.send_packets(&mut self.server);

        Ok(())
    }

    fn shutdown(&mut self) {
        self.transport.disconnect_all(&mut self.server);
    }

    // --- 1. Connection Handling ---

    fn process_net_events(&mut self) {
        while let Some(event) = self.server.get_event() {
            match event {
                ServerEvent::ClientConnected { client_id } => {
                    info!(%client_id, "Client connected");
                    self.client_states.insert(client_id, ClientState::default());
                }
                ServerEvent::ClientDisconnected { client_id, reason } => {
                    info!(%client_id, ?reason, "Client disconnected");
                    // If this client was in a game, he will now be idle.
                    self.client_states.remove(&client_id);
                }
            }
        }
    }

    // --- 2. Message Handling ---

    fn process_client_messages(&mut self) {
        let client_ids = self.server.clients_id();
        for client_id in client_ids {
            while let Some(bytes) = self.server.receive_message(client_id, RELIABLE_CHANNEL_ID) {
                // Decode
                let msg = match decode_client_message(bytes.as_ref()) {
                    Ok(m) => m,
                    Err(e) => {
                        debug!(%client_id, %e, "Failed to decode message");
                        continue;
                    }
                };

                // Route
                if let Err(e) = self.route_message(client_id, msg) {
                    self.send_message(client_id, ServerMessage::Error(e.to_string()));
                }
            }
        }
    }

    fn route_message(&mut self, client_id: ClientId, msg: ClientMessage) -> Result<(), String> {
        // Special Case: GameInput bypasses the heavyweight ClientState DFA for performance
        if let ClientMessage::GameInput(input) = msg {
            if let Some(ClientState::InGame { game_code, .. }) = self.client_states.get(&client_id) {
                if let Some(game) = self.game_manager.games.get_mut(game_code) {
                    game.command_queue.push_back(GameCommand::Input(*client_id, input));
                }
            }
            return Ok(());
        }

        // Standard DFA handling for non-input messages
        let state = self.client_states.get(&client_id).ok_or("No state")?.clone();
        
        // A. DECIDE (Validation)
        let decision = client::handle_message(&state, msg, &mut self.game_manager);

        match decision {
            Ok(Some(event)) => {
                // B. APPLY (State Transition)
                let new_state = state.apply(event.clone());

                // Update mapping if handshake just finished
                if let ClientEvent::HandshakeCompleted { client_id, .. } = event {
                    self.player_to_client.insert(client_id, client_id);
                    // Send ConnectOK
                    self.send_message(client_id, ServerMessage::ConnectOk { session_id: SessionId(0) });
                }
                // Ack Game Join
                if let ClientEvent::GameJoined { game_code } | ClientEvent::GameCreated { game_code } = &event {
                    self.send_message(client_id, ServerMessage::GameJoined { game_code: game_code.clone() });
                }

                self.client_states.insert(client_id, new_state);
                Ok(())
            }
            Ok(None) => Ok(()), // Valid message, no state change
            Err(e) => Err(e),   // Validation failed
        }
    }

    // --- 3. Game Loop ---

    fn tick_games(&mut self, dt: f32) {
        // Iterate over all games
        for game in self.game_manager.games.values_mut() {
            // Run the simulation
            game.tick(dt);

            // If anything interesting happened (events), broadcast it
            if !game.outgoing_events.is_empty() {
                // We need to construct a snapshot for the clients
                let update = match &game.state {
                    GameState::Battle { engine } => GameUpdate {
                        state: engine.state.clone(), // In real app, avoid full clone
                        events: game.outgoing_events.clone()
                            .into_iter()
                            .filter_map(|e| self.map_game_event(e))
                            .collect(),
                    },
                    // If waiting, we send empty snapshots but important events (PlayerJoined)
                    GameState::Waiting { .. } => GameUpdate {
                        state: GameStateSnapshot {
                            players: vec![],
                            projectiles: vec![],
                            time_remaining: 0.0,
                        },
                        events: game.outgoing_events.clone()
                            .into_iter()
                            .filter_map(|e| self.map_game_event(e))
                            .collect(),
                    },
                    _ => continue,
                };

                // Broadcast to all players in this game
                // We find the players by checking who is in the game logic
                // (Optimally Game struct should keep a list of connected IDs)
                let players_in_game = self.get_players_in_game(game);
                
                for pid in players_in_game {
                    if let Some(&cid) = self.player_to_client.get(&pid) {
                        self.send_message(cid, ServerMessage::GameUpdate(update.clone()));
                    }
                }
            }
        }
    }

    // Helper to Map Internal GameEvent -> Protocol GameEvent
    fn map_game_event(&self, e: game::GameEvent) -> Option<common::protocol::GameEvent> {
        use common::protocol::GameEvent as ProtoEvent;
        use game::GameEvent as InternalEvent;
        match e {
            InternalEvent::PlayerJoined(pid) => Some(ProtoEvent::PlayerJoined(pid)),
            InternalEvent::PlayerLeft(pid) => Some(ProtoEvent::PlayerLeft(pid)),
            InternalEvent::GameStarted(map) => Some(ProtoEvent::GameStarted(map)),
            InternalEvent::GameEnded(team) => Some(ProtoEvent::GameEnded(team)),
        }
    }

    fn get_players_in_game(&self, game: &Game) -> Vec<PlayerId> {
        match &game.state {
            GameState::Waiting { players } => players.clone(),
            GameState::Battle { engine } => engine.state.players.iter().map(|p| p.id).collect(),
            GameState::Ended { .. } => vec![], 
        }
    }

    fn send_message(&mut self, client_id: ClientId, message: ServerMessage) {
        if let Ok(payload) = encode_server_message(&message) {
            self.server.send_message(client_id, RELIABLE_CHANNEL_ID, payload);
        }
    }
}