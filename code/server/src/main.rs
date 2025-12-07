mod client;
mod game;
mod game_manager;

use std::{collections::HashMap, net::SocketAddr, net::UdpSocket, time::Duration, time::Instant};

use client::{ClientEvent, ClientState};
use common::codec::{decode_client_message, encode_server_message};
use common::protocol::{
    ClientMessage, GameUpdate, GameStateSnapshot, ServerMessage,
};
use game::{Game, GameCommand, GameState};
use game_manager::GameManager;

use renet::{ClientId, ConnectionConfig, RenetServer, ServerEvent};
use renet_netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig};
use tokio::time::{self, MissedTickBehavior};
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

const SERVER_PORT: u16 = 8080;
const MAX_CLIENTS: usize = 64;
const PROTOCOL_ID: u64 = 0;
const RELIABLE_CHANNEL_ID: u8 = 0;
const TICK_INTERVAL: Duration = Duration::from_micros(33_333); // ≈30 Hz

type AppResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> AppResult<()> {
    init_tracing();

    let mut app = ServerApp::new()?;
    info!("Server listening on port {}", SERVER_PORT);

    let mut ticker = time::interval(TICK_INTERVAL);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Shutting down...");
                break;
            }
            _ = ticker.tick() => {
                if let Err(err) = app.tick() {
                    error!(error = %err, "Tick failed");
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

fn tick(&mut self) -> AppResult<()> {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f32();
        self.last_tick = now;

        self.transport.update(Duration::from_secs_f32(dt), &mut self.server)?;
        self.server.update(Duration::from_secs_f32(dt));

        self.process_net_events();
        self.process_client_messages();

        let updates = self.game_manager.tick(dt);
        
        for (recipients, update) in updates {
            // Encode once, send bytes to many.
            if let Ok(payload) = encode_server_message(&ServerMessage::GameUpdate(update)) {
                for client_id in recipients {
                    // TODO: implement clean from
                    let renet_id = renet::ClientId::from_raw(client_id.0);
                    self.server.send_message(renet_id, RELIABLE_CHANNEL_ID, payload.clone());
                }
            }
        }

        self.transport.send_packets(&mut self.server);

        Ok(())
    }

    fn shutdown(&mut self) {
        self.transport.disconnect_all(&mut self.server);
    }

    fn process_net_events(&mut self) {
        while let Some(event) = self.server.get_event() {
            match event {
                ServerEvent::ClientConnected { client_id } => {
                    info!(%client_id, "Client connected");
                    self.client_states.insert(client_id, ClientState::default());
                }
                ServerEvent::ClientDisconnected { client_id, reason } => {
                    info!(%client_id, ?reason, "Client disconnected");
                    // If this client was in a game, he will be idle and unable to reconnect.
                    self.client_states.remove(&client_id);
                }
            }
        }
    }

    fn process_client_messages(&mut self) {
        let client_ids = self.server.clients_id();
        for client_id in client_ids {
            while let Some(bytes) = self.server.receive_message(client_id, RELIABLE_CHANNEL_ID) {
                let msg = match decode_client_message(bytes.as_ref()) {
                    Ok(m) => m,
                    Err(e) => {
                        debug!(%client_id, %e, "Failed to decode message");
                        continue;
                    }
                };

                if let Err(e) = self.route_message(client_id, msg) {
                    self.send_message(client_id, ServerMessage::Error(e.to_string()));
                }
            }
        }
    }

fn route_message(&mut self, renet_id: ClientId, msg: ClientMessage) -> Result<(), String> {
        // Convert Renet ID -> Common ID
        // TODO: implement from trait
        let common_id = common::protocol::ClientId(renet_id.raw());

        // FAST PATH: Game Input
        if let ClientMessage::GameInput(input) = msg {
            if let Some(ClientState::InGame { game_code, .. }) = self.client_states.get(&renet_id) {
                self.game_manager.handle_input(game_code, common_id, input);
            }
            return Ok(());
        }

        let state = self.client_states.get(&renet_id).ok_or("No state")?.clone();
        let decision = client::handle_message(&state, msg, &mut self.game_manager, common_id);

        match decision {
            Ok(Some(event)) => {
                let new_state = state.apply(event.clone());

                match &event {
                    ClientEvent::HandshakeCompleted { .. } => {
                        self.send_message(renet_id, ServerMessage::ConnectOk);
                    }
                    ClientEvent::GameJoined { game_code } | ClientEvent::GameCreated { game_code } => {
                        self.send_message(renet_id, ServerMessage::GameJoined { game_code: game_code.clone() });
                    }
                    _ => {}
                }

                self.client_states.insert(renet_id, new_state);
                Ok(())
            }
            Ok(None) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn send_message(&mut self, client_id: ClientId, message: ServerMessage) {
        if let Ok(payload) = encode_server_message(&message) {
            self.server.send_message(client_id, RELIABLE_CHANNEL_ID, payload);
        }
    }
}