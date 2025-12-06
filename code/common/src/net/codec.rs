use bincode::error::{DecodeError, EncodeError};
use bincode::{decode_from_slice, encode_to_vec};

use super::protocol::{ClientMessage, ServerMessage};

/// Shared binary configuration used by both server and client.
fn bincode_config() -> impl bincode::config::Config {
    bincode::config::standard()
}

/// Serialize a [`ServerMessage`] into a byte vector suitable for transport.
pub fn encode_server_message(message: &ServerMessage) -> Result<Vec<u8>, EncodeError> {
    encode_to_vec(message, bincode_config())
}

/// Deserialize a [`ClientMessage`] from a byte slice delivered by the transport.
pub fn decode_client_message(bytes: &[u8]) -> Result<ClientMessage, DecodeError> {
    let (message, _) = decode_from_slice(bytes, bincode_config())?;
    Ok(message)
}

/// Serialize a [`ClientMessage`] for sending to the server.
pub fn encode_client_message(message: &ClientMessage) -> Result<Vec<u8>, EncodeError> {
    encode_to_vec(message, bincode_config())
}

/// Deserialize a [`ServerMessage`] received from the server.
pub fn decode_server_message(bytes: &[u8]) -> Result<ServerMessage, DecodeError> {
    let (message, _) = decode_from_slice(bytes, bincode_config())?;
    Ok(message)
}
