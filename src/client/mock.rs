use crate::protocol::{
    CLIENT_COMMAND_CHANNEL, CLIENT_INPUT_CHANNEL, ClientMessage, PROTOCOL_ID,
    SERVER_RELIABLE_CHANNEL, SERVER_STATE_CHANNEL, ServerMessage, connection_config,
    deserialize_server_message, serialize_client_message,
};
use anyhow::{Context, Result};
use bytes::Bytes;
use rand::random;
use renet::RenetClient;
use renet_netcode::{ClientAuthentication, NetcodeClientTransport};
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

pub async fn run(args: ClientArgs) -> Result<()> {
    let mut client = RenetClient::new(connection_config());
    let socket = UdpSocket::bind("0.0.0.0:0").context("bind local udp")?;
    let auth = ClientAuthentication::Unsecure {
        protocol_id: PROTOCOL_ID,
        client_id: args.client_id,
        server_addr: args.server_addr,
        user_data: None,
    };
    let mut transport = NetcodeClientTransport::new(Duration::ZERO, auth, socket)?;

    let mut interval = tokio::time::interval(Duration::from_millis(16));
    let mut handshake_sent = false;
    let mut connect_ack = false;
    let mut room_joined = false;
    let mut room_requested = false;
    let start = Instant::now();
    let mut tick_id = 0u64;
    let mut last_input = Instant::now();

    loop {
        interval.tick().await;
        let dt = Duration::from_millis(16);
        client.update(dt);
        transport.update(dt, &mut client).ok();
        transport.send_packets(&mut client).ok();

        if client.is_connected() && !handshake_sent {
            send_client_message(
                &mut client,
                ClientMessage::Connect {
                    nickname: args.nickname.clone(),
                    client_version: Some("mock-client".into()),
                },
            )?;
            handshake_sent = true;
        }

        process_server_messages(&mut client, &mut connect_ack, &mut room_joined);

        if connect_ack && !room_requested {
            match &args.room_code {
                Some(code) => {
                    send_client_message(
                        &mut client,
                        ClientMessage::RoomJoin {
                            room_code: code.clone(),
                        },
                    )?;
                }
                None => {
                    send_client_message(&mut client, ClientMessage::RoomCreate)?;
                }
            }
            room_requested = true;
        }

        if room_joined && last_input.elapsed() >= Duration::from_millis(150) {
            tick_id = tick_id.wrapping_add(1);
            send_client_message(
                &mut client,
                ClientMessage::Input {
                    tick_id,
                    payload: vec![0, 1, 2, 3],
                },
            )?;
            client.send_message(CLIENT_INPUT_CHANNEL, Bytes::from(vec![tick_id as u8; 8]));
            last_input = Instant::now();
        }

        if start.elapsed() > Duration::from_secs(30) {
            break;
        }
    }

    send_client_message(&mut client, ClientMessage::Disconnect)?;
    Ok(())
}

pub fn init_tracing() {
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();
}

pub struct ClientArgs {
    pub nickname: String,
    pub server_addr: SocketAddr,
    pub room_code: Option<String>,
    pub client_id: u64,
}

impl ClientArgs {
    pub fn parse() -> Result<Self> {
        let mut args = std::env::args().skip(1);
        let nickname = args.next().unwrap_or_else(|| "Mocky".to_string());
        let server_addr = args
            .next()
            .unwrap_or_else(|| "127.0.0.1:5000".to_string())
            .parse()
            .context("invalid server address")?;
        let room_code = args.next();
        Ok(Self {
            nickname,
            server_addr,
            room_code,
            client_id: random(),
        })
    }
}

fn process_server_messages(
    client: &mut RenetClient,
    connect_ack: &mut bool,
    room_joined: &mut bool,
) {
    while let Some(bytes) = client.receive_message(SERVER_RELIABLE_CHANNEL) {
        if let Ok(message) = deserialize_server_message(bytes.as_ref()) {
            match message {
                ServerMessage::ConnectOk => {
                    *connect_ack = true;
                }
                ServerMessage::RoomJoinOk { room_state } => {
                    *room_joined = true;
                    tracing::info!("joined room {}", room_state.room_code);
                }
                ServerMessage::RoomCreateOk { room_code } => {
                    tracing::info!("created room {}", room_code);
                }
                ServerMessage::RoomDelta { delta } => {
                    tracing::debug!("room delta {:?}", delta.state_id);
                }
                ServerMessage::ServerError { code, message } => {
                    tracing::warn!("server error {}: {}", code, message);
                }
                _ => {}
            }
        }
    }

    while let Some(bytes) = client.receive_message(SERVER_STATE_CHANNEL) {
        if let Ok(message) = deserialize_server_message(bytes.as_ref()) {
            match message {
                ServerMessage::GameSnapshot { snapshot } => {
                    tracing::info!("snapshot {} tick {}", snapshot.game_id, snapshot.tick_id);
                }
                ServerMessage::GameDelta { delta } => {
                    tracing::debug!("delta {} tick {}", delta.game_id, delta.tick_id);
                }
                _ => {}
            }
        }
    }
}

fn send_client_message(client: &mut RenetClient, message: ClientMessage) -> Result<()> {
    let bytes = serialize_client_message(&message)?;
    client.send_message(CLIENT_COMMAND_CHANNEL, Bytes::from(bytes));
    Ok(())
}
