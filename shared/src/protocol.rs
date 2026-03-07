use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    MoveIntent(MoveIntent),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MoveIntent {
    pub target_x: f32,
    pub target_y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    PlayerState(PlayerState),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PlayerState {
    pub player_id: u64,
    pub x: f32,
    pub y: f32,
}

pub fn encode_client_message(message: &ClientMessage) -> Result<Vec<u8>, bincode::Error> {
    bincode::serialize(message)
}

pub fn decode_client_message(payload: &[u8]) -> Result<ClientMessage, bincode::Error> {
    bincode::deserialize(payload)
}

pub fn encode_server_message(message: &ServerMessage) -> Result<Vec<u8>, bincode::Error> {
    bincode::serialize(message)
}

pub fn decode_server_message(payload: &[u8]) -> Result<ServerMessage, bincode::Error> {
    bincode::deserialize(payload)
}
