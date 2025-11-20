mod game_loop;
mod state;

use self::game_loop::{GameLoopEvent, MockGameLoop};
use self::state::{RoomBroadcast, SharedState};
use crate::protocol::{
    CLIENT_COMMAND_CHANNEL, CLIENT_INPUT_CHANNEL, ClientMessage, PROTOCOL_ID,
    SERVER_RELIABLE_CHANNEL, SERVER_STATE_CHANNEL, ServerMessage, connection_config,
    deserialize_client_message, serialize_server_message,
};
use anyhow::{Context, Result};
use bytes::Bytes;
use renet::{ClientId, RenetServer, ServerEvent};
use renet_netcode::{
    NetcodeServerTransport, ServerAuthentication, ServerConfig as NetcodeServerConfig,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{debug, error, info, warn};

pub struct ServerOptions {
    pub bind_addr: SocketAddr,
    pub max_clients: usize,
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:5000".parse().expect("valid socket"),
            max_clients: 64,
        }
    }
}

pub async fn run(options: ServerOptions) -> Result<()> {
    info!("starting NeuroBlasters server on {}", options.bind_addr);
    let state = SharedState::new();
    let (loop_tx, mut loop_rx) = tokio::sync::mpsc::unbounded_channel();
    let _loop_handle = MockGameLoop::spawn(state.clone(), loop_tx);

    let socket = std::net::UdpSocket::bind(options.bind_addr).context("bind UDP socket")?;
    let netcode_config = NetcodeServerConfig {
        current_time: Duration::ZERO,
        max_clients: options.max_clients,
        protocol_id: PROTOCOL_ID,
        public_addresses: vec![options.bind_addr],
        authentication: ServerAuthentication::Unsecure,
    };

    let connection_config = connection_config();
    let mut renet_server = RenetServer::new(connection_config);
    let mut transport = NetcodeServerTransport::new(netcode_config, socket)?;

    let mut last_tick = Instant::now();
    let mut snapshots: HashMap<String, crate::protocol::GameSnapshot> = HashMap::new();
    let mut interval = tokio::time::interval(Duration::from_millis(16));

    loop {
        interval.tick().await;
        let now = Instant::now();
        let delta = now - last_tick;
        last_tick = now;
        renet_server.update(delta);
        transport.update(delta, &mut renet_server)?;

        process_events(&mut renet_server, &state);
        process_messages(&mut renet_server, &state, &mut snapshots)?;
        flush_game_events(&mut renet_server, &mut snapshots, &mut loop_rx);

        transport.send_packets(&mut renet_server);
    }
}

fn process_events(server: &mut RenetServer, state: &Arc<SharedState>) {
    while let Some(event) = server.get_event() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                state.register_client(client_id);
                info!("client {} connected", client_id);
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                info!("client {} disconnected: {:?}", client_id, reason);
                if let Some(broadcast) = state.unregister_client(client_id) {
                    send_room_delta(server, broadcast);
                }
            }
        }
    }
}

fn process_messages(
    server: &mut RenetServer,
    state: &Arc<SharedState>,
    snapshots: &mut HashMap<String, crate::protocol::GameSnapshot>,
) -> Result<()> {
    for client_id in server.clients_id() {
        while let Some(bytes) = server.receive_message(client_id, CLIENT_COMMAND_CHANNEL) {
            match deserialize_client_message(bytes.as_ref()) {
                Ok(message) => handle_command(server, state, snapshots, client_id, message)?,
                Err(err) => {
                    warn!("failed to deserialize message from {}: {}", client_id, err);
                    send_reliable(
                        server,
                        client_id,
                        ServerMessage::ServerError {
                            code: 4001,
                            message: "invalid payload".to_string(),
                        },
                    );
                }
            }
        }

        while let Some(bytes) = server.receive_message(client_id, CLIENT_INPUT_CHANNEL) {
            handle_input_packet(server, client_id, bytes);
        }
    }
    Ok(())
}

fn handle_command(
    server: &mut RenetServer,
    state: &Arc<SharedState>,
    snapshots: &mut HashMap<String, crate::protocol::GameSnapshot>,
    client_id: ClientId,
    message: ClientMessage,
) -> Result<()> {
    match message {
        ClientMessage::Connect { nickname, .. } => match state.set_nickname(client_id, nickname) {
            Ok(_) => send_reliable(server, client_id, ServerMessage::ConnectOk),
            Err(err) => send_reliable(
                server,
                client_id,
                ServerMessage::ConnectError {
                    error: format!("{}", err),
                },
            ),
        },
        ClientMessage::Disconnect => {
            server.disconnect(client_id);
        }
        ClientMessage::RoomCreate => match state.create_room(client_id) {
            Ok(outcome) => {
                send_reliable(
                    server,
                    client_id,
                    ServerMessage::RoomCreateOk {
                        room_code: outcome.room_code.clone(),
                    },
                );
                send_reliable(
                    server,
                    client_id,
                    ServerMessage::RoomJoinOk {
                        room_state: outcome.state,
                    },
                );
            }
            Err(err) => send_reliable(
                server,
                client_id,
                ServerMessage::RoomCreateError {
                    error: err.to_string(),
                },
            ),
        },
        ClientMessage::RoomJoin { room_code } => match state.join_room(client_id, &room_code) {
            Ok(outcome) => {
                send_reliable(
                    server,
                    client_id,
                    ServerMessage::RoomJoinOk {
                        room_state: outcome.state,
                    },
                );
                if let Some(broadcast) = outcome.broadcast {
                    send_room_delta(server, broadcast);
                }
            }
            Err(err) => send_reliable(
                server,
                client_id,
                ServerMessage::RoomJoinError {
                    error: err.to_string(),
                },
            ),
        },
        ClientMessage::RoomLeave => match state.leave_room(client_id) {
            Ok(outcome) => {
                send_reliable(server, client_id, ServerMessage::RoomLeaveOk);
                if let Some(broadcast) = outcome.broadcast {
                    send_room_delta(server, broadcast);
                }
            }
            Err(err) => send_reliable(
                server,
                client_id,
                ServerMessage::RoomLeaveError {
                    error: err.to_string(),
                },
            ),
        },
        ClientMessage::Input { tick_id, payload } => {
            debug!(
                "input from {} tick {} ({} bytes)",
                client_id,
                tick_id,
                payload.len()
            );
        }
        ClientMessage::ResyncRequest { game_id } => {
            if let Some(snapshot) = snapshots.get(&game_id) {
                send_state(
                    server,
                    client_id,
                    ServerMessage::GameSnapshot {
                        snapshot: snapshot.clone(),
                    },
                );
            } else {
                send_reliable(
                    server,
                    client_id,
                    ServerMessage::ServerError {
                        code: 4404,
                        message: format!("game {} unknown", game_id),
                    },
                );
            }
        }
    }
    Ok(())
}

fn handle_input_packet(server: &mut RenetServer, client_id: ClientId, bytes: Bytes) {
    if bytes.len() > 1024 {
        send_reliable(
            server,
            client_id,
            ServerMessage::InputError {
                tick_id: 0,
                error: "input payload too large".to_string(),
            },
        );
        return;
    }
    debug!(
        "received {} bytes of input data from {}",
        bytes.len(),
        client_id
    );
}

fn flush_game_events(
    server: &mut RenetServer,
    snapshots: &mut HashMap<String, crate::protocol::GameSnapshot>,
    loop_rx: &mut UnboundedReceiver<GameLoopEvent>,
) {
    while let Ok(event) = loop_rx.try_recv() {
        match event {
            GameLoopEvent::Snapshot(snapshot) => {
                snapshots.insert(snapshot.game_id.clone(), snapshot.clone());
                broadcast_state(server, ServerMessage::GameSnapshot { snapshot });
            }
            GameLoopEvent::Delta(delta) => {
                broadcast_state(server, ServerMessage::GameDelta { delta });
            }
        }
    }
}

fn send_reliable(server: &mut RenetServer, client_id: ClientId, message: ServerMessage) {
    match serialize_server_message(&message) {
        Ok(payload) => {
            server.send_message(client_id, SERVER_RELIABLE_CHANNEL, Bytes::from(payload))
        }
        Err(err) => error!("failed to serialize server message: {}", err),
    }
}

fn send_state(server: &mut RenetServer, client_id: ClientId, message: ServerMessage) {
    match serialize_server_message(&message) {
        Ok(payload) => server.send_message(client_id, SERVER_STATE_CHANNEL, Bytes::from(payload)),
        Err(err) => error!("failed to serialize state message: {}", err),
    }
}

fn broadcast_state(server: &mut RenetServer, message: ServerMessage) {
    match serialize_server_message(&message) {
        Ok(payload) => server.broadcast_message(SERVER_STATE_CHANNEL, Bytes::from(payload)),
        Err(err) => error!("failed to broadcast state message: {}", err),
    }
}

fn send_room_delta(server: &mut RenetServer, broadcast: RoomBroadcast) {
    match serialize_server_message(&ServerMessage::RoomDelta {
        delta: broadcast.delta,
    }) {
        Ok(payload) => {
            let bytes = Bytes::from(payload);
            for client_id in broadcast.recipients {
                server.send_message(client_id, SERVER_RELIABLE_CHANNEL, bytes.clone());
            }
        }
        Err(err) => error!("failed to encode room delta: {}", err),
    }
}
