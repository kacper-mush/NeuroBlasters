use common::{API_VERSION, ApiVersion, ConnectError, GameCode, ServerMessage, SessionId};
use rand::RngCore;
use renet::ClientId;

use crate::ServerApp;

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session_id: SessionId,
    pub nickname: String,
    pub game_code: Option<GameCode>,
}

impl ServerApp {
    pub(super) fn handle_connect_message(
        &mut self,
        client_id: ClientId,
        api_version: u16,
        nickname: String,
    ) -> Result<(), ConnectError> {
        let requested_version = ApiVersion(api_version);

        if requested_version != API_VERSION {
            self.server.disconnect(client_id);
            return Err(ConnectError::ApiVersionMismatch {
                requested: requested_version.0,
                expected: API_VERSION.0,
            });
        }

        if let Some(existing) = self.sessions.get(&client_id) {
            return Err(ConnectError::DuplicateHandshake {
                session_id: existing.session_id,
            });
        }

        let session_id = self.next_session_id();
        let nickname = nickname.trim().to_owned();
        tracing::info!(
            client_id = %client_id,
            session_id = session_id.0,
            nickname = %nickname,
            "handshake successful"
        );

        self.sessions.insert(
            client_id,
            SessionInfo {
                session_id,
                nickname,
                game_code: None,
            },
        );

        self.send_message(client_id, ServerMessage::ConnectOk { session_id });
        Ok(())
    }

    pub(super) fn handle_disconnect_request(
        &mut self,
        client_id: ClientId,
    ) -> Result<(), ConnectError> {
        tracing::debug!(client_id = %client_id, "client requested disconnect");
        if let Some(game_code) = self.detach_client_from_game(client_id) {
            self.broadcast_game_update(&game_code);
        }

        self.sessions.remove(&client_id);
        self.server.disconnect(client_id);
        Ok(())
    }

    fn next_session_id(&mut self) -> SessionId {
        SessionId(self.rng.next_u64())
    }
}
