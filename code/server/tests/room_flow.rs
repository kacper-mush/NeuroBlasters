use std::{
    net::{SocketAddr, UdpSocket},
    process::Stdio,
    time::{Duration, Instant, SystemTime},
};

use common::{
    API_VERSION, ClientMessage, RoomEvent, ServerMessage, decode_server_message,
    encode_client_message,
};
use rand::random;
use renet::{ConnectionConfig, RenetClient};
use renet_netcode::{ClientAuthentication, NetcodeClientTransport};
use tokio::{process::Command, time::sleep};

const RELIABLE_CHANNEL_ID: u8 = 0;
const PROTOCOL_ID: u64 = 0;
const SERVER_ADDR: &str = "127.0.0.1:8080";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn room_create_flow_end_to_end() -> Result<(), Box<dyn std::error::Error>> {
    let server_path = env!("CARGO_BIN_EXE_server");
    let mut server = Command::new(server_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    sleep(Duration::from_millis(200)).await;

    let server_addr: SocketAddr = SERVER_ADDR.parse()?;
    let mut client = RenetClient::new(ConnectionConfig::default());
    let socket = UdpSocket::bind("127.0.0.1:0")?;
    socket.set_nonblocking(true)?;

    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let authentication = ClientAuthentication::Unsecure {
        server_addr,
        client_id: random(),
        user_data: None,
        protocol_id: PROTOCOL_ID,
    };
    let mut transport = NetcodeClientTransport::new(current_time, authentication, socket)?;

    let mut last_tick = Instant::now();
    let mut connect_sent = false;
    let mut connect_ok = false;
    let mut room_create_sent = false;
    let mut room_create_ok = false;
    let mut update_received = false;

    for _ in 0..400 {
        let now = Instant::now();
        let delta = now - last_tick;
        last_tick = now;

        client.update(delta);
        transport.update(delta, &mut client)?;

        if client.is_connected() {
            if !connect_sent {
                send_message(
                    &mut client,
                    ClientMessage::Connect {
                        api_version: API_VERSION.0,
                        nickname: "tester".into(),
                    },
                )?;
                connect_sent = true;
            } else if connect_ok && !room_create_sent {
                send_message(&mut client, ClientMessage::RoomCreate)?;
                room_create_sent = true;
            }
        }

        while let Some(payload) = client.receive_message(RELIABLE_CHANNEL_ID) {
            match decode_server_message(payload.as_ref())? {
                ServerMessage::ConnectOk { .. } => {
                    connect_ok = true;
                }
                ServerMessage::RoomCreateOk { room_code } => {
                    assert!(!room_code.0.is_empty(), "room code should not be empty");
                    room_create_ok = true;
                }
                ServerMessage::RoomUpdate { update } => {
                    if update
                        .events
                        .iter()
                        .any(|event| matches!(event, RoomEvent::PlayerJoined { .. }))
                    {
                        update_received = true;
                    }
                }
                _ => {}
            }
        }

        transport.send_packets(&mut client)?;

        if connect_ok && room_create_ok && update_received {
            break;
        }

        sleep(Duration::from_millis(10)).await;
    }

    let _ = server.kill().await;

    assert!(connect_ok, "did not receive ConnectOk");
    assert!(room_create_ok, "did not receive RoomCreateOk");
    assert!(update_received, "did not receive RoomUpdate");

    Ok(())
}

fn send_message(
    client: &mut RenetClient,
    message: ClientMessage,
) -> Result<(), Box<dyn std::error::Error>> {
    let payload = encode_client_message(&message)?;
    client.send_message(RELIABLE_CHANNEL_ID, payload);
    Ok(())
}
