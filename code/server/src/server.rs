use std::{net::SocketAddr, net::UdpSocket, time::Duration, time::Instant};

use common::codec::{decode_client_message, encode_server_message};
use common::protocol::ServerMessage;

use crate::server_logic::ServerLogic;

use renet::{ClientId, ConnectionConfig, RenetServer, ServerEvent};
use renet_netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig};
use tracing::{debug, info};

const SERVER_PORT: u16 = 8080;
const MAX_CLIENTS: usize = 64;
const PROTOCOL_ID: u64 = 0;
const RELIABLE_CHANNEL_ID: u8 = 0;

type AppResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct ServerApp {
    server: RenetServer,
    transport: NetcodeServerTransport,

    logic: ServerLogic,

    last_tick: Instant,
}

impl ServerApp {
    pub fn new() -> AppResult<Self> {
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

        info!("Server listening on port {}", SERVER_PORT);

        Ok(Self {
            server,
            transport,
            logic: ServerLogic::new(),
            last_tick: Instant::now(),
        })
    }

    pub fn tick(&mut self) -> AppResult<()> {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f32();
        self.last_tick = now;

        self.transport
            .update(Duration::from_secs_f32(dt), &mut self.server)?;
        self.server.update(Duration::from_secs_f32(dt));

        self.process_net_events();
        self.process_client_messages();

        let updates = self.logic.game_manager_mut().tick(dt);

        for (recipients, update) in updates {
            // Encode once, send bytes to many.
            if let Ok(payload) = encode_server_message(&ServerMessage::GameUpdate(update)) {
                for client_id in recipients {
                    self.server
                        .send_message(client_id, RELIABLE_CHANNEL_ID, payload.clone());
                }
            }
        }

        self.transport.send_packets(&mut self.server);

        Ok(())
    }

    pub fn shutdown(&mut self) {
        self.transport.disconnect_all(&mut self.server);
    }

    fn process_net_events(&mut self) {
        while let Some(event) = self.server.get_event() {
            match event {
                ServerEvent::ClientConnected { client_id } => {
                    info!(%client_id, "Client connected");
                }
                ServerEvent::ClientDisconnected { client_id, reason } => {
                    info!(%client_id, ?reason, "Client disconnected");
                    self.logic.on_disconnect(client_id);
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

                let message = self.logic.handle_message(client_id, msg);

                match message {
                    Ok(Some(message)) => self.send_message(client_id, message),
                    Ok(None) => (),
                    Err(e) => {
                        debug!(%client_id, %e, "Failed to handle message");
                        continue;
                    }
                }
            }
        }
    }

    fn send_message(&mut self, client_id: ClientId, message: ServerMessage) {
        if let Ok(payload) = encode_server_message(&message) {
            self.server
                .send_message(client_id, RELIABLE_CHANNEL_ID, payload);
        }
    }
}
