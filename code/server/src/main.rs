mod connection;
mod client;
use client::{ClientState, ClientEvent, GameManager};

use std::{collections::HashMap, net::SocketAddr, net::UdpSocket, time::Duration, time::Instant};

use common::codec::{decode_client_message, encode_server_message};
use common::protocol::{ClientMessage, ConnectError, ServerError, ServerMessage};
use connection::SessionInfo;
use rand::{SeedableRng, rngs::StdRng};
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
                info!("received Ctrl+C, shutting down immediately");
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
    /// Only authenticated (handshaken) clients are stored here.
    client_states: HashMap<ClientId, ClientState>,
    game_states: HashMap<GameId, GameState>,
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
            // TODO: Add authentication.
            authentication: ServerAuthentication::Unsecure,
        };

        let socket = UdpSocket::bind(public_addr)?;
        let transport = NetcodeServerTransport::new(server_config, socket)?;
        let server = RenetServer::new(ConnectionConfig::default());

        Ok(Self {
            server,
            transport,
            sessions: HashMap::new(),
            rng: StdRng::from_os_rng(),
            last_tick: Instant::now(),
        })
    }

    fn public_addresses(&self) -> Vec<SocketAddr> {
        self.transport.addresses()
    }

    fn tick(&mut self) -> AppResult<()> {
        let now = Instant::now();
        let delta = now - self.last_tick;
        self.last_tick = now;

        // Recieve packets from clients.
        self.transport.update(delta, &mut self.server)?;
        // Advance the reliable-UDP simulation.
        self.server.update(delta);
        self.process_events();
        self.process_messages();
        // Send the queued packets to the clients.
        self.transport.send_packets(&mut self.server);

        Ok(())
    }

    fn shutdown(&mut self) {
        info!("disconnecting all clients");
        self.transport.disconnect_all(&mut self.server);
        self.sessions.clear();
    }

fn process_events(&mut self) {
        while let Some(event) = self.server.get_event() {
            match event {
                ServerEvent::ClientConnected { client_id } => {
                    info!(client_id = %client_id, "Physical connection established");
                    // Initialize new client in Handshaking state
                    self.client_states.insert(client_id, ClientState::default());
                }
                ServerEvent::ClientDisconnected { client_id, reason } => {
                    info!(client_id = %client_id, ?reason, "Physical disconnect");
                    // TODO: Clean up game if they were in one
                    self.client_states.remove(&client_id);
                }
            }
        }
    }

    /// Deserialize and handle client messages.
    fn process_messages(&mut self) {
        let client_ids = self.server.clients_id();
        for client_id in client_ids {
            while let Some(bytes) = self.server.receive_message(client_id, RELIABLE_CHANNEL_ID) {
                let result = decode_client_message(bytes.as_ref())
                    .map_err(|err| {
                        debug!(client_id = %client_id, %err, "failed to decode client message");
                        ServerError::General
                    })
                    .and_then(|msg| self.route_client_message(client_id, msg));

                if let Err(err) = result {
                    self.send_error(client_id, err);
                }
            }
        }
    }

    fn route_client_message(
            &mut self,
            client_id: ClientId,
            message: ClientMessage,
        ) -> Result<(), ServerError> {
            
            // 1. Get the Client's current state
            // If they aren't in the map, something went wrong with connection events.
            let Some(current_state) = self.client_states.get(&client_id).cloned() else {
                return Err(ServerError::General);
            };

            // 2. DECIDE: logic check (Validation)
            let decision = client::handle_message(
                &current_state, 
                message, 
                &mut self.game_manager
            );

            match decision {
                Ok(Some(event)) => {
                    // 3. APPLY: Update the state
                    let new_state = current_state.apply(event.clone());
                    self.client_states.insert(client_id, new_state);
                    
                    // 4. RESPOND: Tell the client what happened
                    // You might want to convert ClientEvent -> ServerMessage here.
                    // For example:
                    match event {
                        ClientEvent::HandshakeCompleted { .. } => {
                            // send ServerMessage::ConnectOk...
                        }
                        ClientEvent::GameJoined { .. } => {
                            // send ServerMessage::GameJoinOk...
                        }
                        _ => {}
                    }
                },
                Ok(None) => {
                    // Logic valid, but no state change (e.g. Chat)
                },
                Err(error_msg) => {
                    // Logic failed (Game Full, Wrong Password, etc)
                    // send ServerMessage::Error(error_msg)...
                }
            }

            Ok(())
        }
    }

    fn send_error(&mut self, client_id: ClientId, error: ServerError) {
        self.send_message(client_id, ServerMessage::Error(error));
    }

    fn send_message(&mut self, client_id: ClientId, message: ServerMessage) {
        match encode_server_message(&message) {
            Ok(payload) => self
                .server
                .send_message(client_id, RELIABLE_CHANNEL_ID, payload),
            Err(err) => {
                error!(client_id = %client_id, %err, ?message, "failed to encode server message");
            }
        }
    }
}
