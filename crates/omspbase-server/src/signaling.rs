// WebSocket signaling server with room management

use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::routing::get;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

// ── Query parameters ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    /// Room identifier (required)
    pub room: String,
    /// Peer role: "host" (publisher) or "remote" (subscriber)
    #[serde(default = "default_role")]
    pub role: String,
}

fn default_role() -> String {
    "remote".to_string()
}

// ── Signaling messages ──────────────────────────────────────────────────────

/// JSON signaling message — enriched with peer context for routing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SignalingMessage {
    /// PSK authentication (first message required if PSK is configured)
    #[serde(rename = "auth")]
    Auth { token: String },

    /// Server confirms authentication
    #[serde(rename = "auth_ok")]
    AuthOk,

    /// SDP offer
    #[serde(rename = "offer")]
    Offer { sdp: String },

    /// SDP answer
    #[serde(rename = "answer")]
    Answer { sdp: String },

    /// ICE candidate
    #[serde(rename = "ice")]
    Ice {
        candidate: String,
        #[serde(rename = "sdpMid")]
        sdp_mid: String,
        #[serde(rename = "sdpMLineIndex")]
        sdp_m_line_index: u16,
    },

    /// Peer joined notification (server → clients)
    #[serde(rename = "peer_joined")]
    PeerJoined { role: String, id: String },

    /// Peer left notification (server → clients)
    #[serde(rename = "peer_left")]
    PeerLeft { role: String, id: String },
}

// ── Room state ──────────────────────────────────────────────────────────────

/// Per-room broadcast channel and metadata
#[derive(Debug)]
struct RoomState {
    /// Broadcast all messages within this room
    tx: broadcast::Sender<String>,
    /// Count of connected hosts
    host_count: usize,
    /// Count of connected remotes
    remote_count: usize,
}

impl RoomState {
    fn new() -> Self {
        let (tx, _) = broadcast::channel::<String>(64);
        RoomState {
            tx,
            host_count: 0,
            remote_count: 0,
        }
    }
}

// ── Signaling server ────────────────────────────────────────────────────────

/// Shared signaling state — manages rooms and peer connections
#[derive(Clone)]
pub struct SignalingServer {
    /// Map of room_id → RoomState
    rooms: Arc<RwLock<HashMap<String, RoomState>>>,
}

impl SignalingServer {
    pub fn new() -> Self {
        SignalingServer {
            rooms: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get broadcast sender for a room, creating if absent
    fn get_or_create_room(&self, room_id: &str) -> broadcast::Sender<String> {
        let mut rooms = self.rooms.write().unwrap();
        rooms
            .entry(room_id.to_string())
            .or_insert_with(RoomState::new)
            .tx
            .clone()
    }

    /// Increment peer count for role in a room
    fn join_room(&self, room_id: &str, role: &str) {
        let mut rooms = self.rooms.write().unwrap();
        let room = rooms
            .entry(room_id.to_string())
            .or_insert_with(RoomState::new);
        match role {
            "host" => room.host_count += 1,
            _ => room.remote_count += 1,
        }
        tracing::info!(
            "Room {}: {} joined (hosts={}, remotes={})",
            room_id,
            role,
            room.host_count,
            room.remote_count
        );
    }

    /// Decrement peer count for role in a room; remove room if empty
    fn leave_room(&self, room_id: &str, role: &str) {
        let mut rooms = self.rooms.write().unwrap();
        if let Some(room) = rooms.get_mut(room_id) {
            match role {
                "host" => room.host_count = room.host_count.saturating_sub(1),
                _ => room.remote_count = room.remote_count.saturating_sub(1),
            }
            tracing::info!(
                "Room {}: {} left (hosts={}, remotes={})",
                room_id,
                role,
                room.host_count,
                room.remote_count
            );
            if room.host_count == 0 && room.remote_count == 0 {
                rooms.remove(room_id);
                tracing::info!("Room {} removed (empty)", room_id);
            }
        }
    }

    /// Snapshot of active hosts for monitoring
    pub fn active_hosts(&self) -> Vec<String> {
        let rooms = self.rooms.read().unwrap();
        rooms
            .iter()
            .filter(|(_, r)| r.host_count > 0)
            .map(|(id, r)| format!("room={} hosts={}", id, r.host_count))
            .collect()
    }

    /// Snapshot of active remotes for monitoring
    pub fn active_remotes(&self) -> Vec<String> {
        let rooms = self.rooms.read().unwrap();
        rooms
            .iter()
            .filter(|(_, r)| r.remote_count > 0)
            .map(|(id, r)| format!("room={} remotes={}", id, r.remote_count))
            .collect()
    }

    /// Snapshot of room count, total hosts, total remotes for metrics
    pub fn metrics_snapshot(&self) -> (usize, usize, usize) {
        let rooms = self.rooms.read().unwrap();
        let room_count = rooms.len();
        let host_count = rooms.values().map(|r| r.host_count).sum();
        let remote_count = rooms.values().map(|r| r.remote_count).sum();
        (room_count, host_count, remote_count)
    }
}

// ── Router ──────────────────────────────────────────────────────────────────

pub fn signaling_router(server: SignalingServer) -> Router {
    Router::new()
        .route("/ws", get(ws_handler))
        .with_state(server)
}

// ── WebSocket handler ───────────────────────────────────────────────────────

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(server): State<SignalingServer>,
    Query(params): Query<WsQuery>,
) -> impl IntoResponse {
    let room_id = params.room;
    let role = params.role;
    ws.on_upgrade(move |socket| handle_socket(socket, server, room_id, role))
}

async fn handle_socket(
    socket: WebSocket,
    server: SignalingServer,
    room_id: String,
    role: String,
) {
    let (mut sender, mut receiver) = socket.split();

    // Get or create room broadcast channel
    let tx = server.get_or_create_room(&room_id);
    let mut rx = tx.subscribe();
    server.join_room(&room_id, &role);

    // Notify peers about the new connection
    let join_msg = serde_json::to_string(&SignalingMessage::PeerJoined {
        role: role.clone(),
        id: room_id.clone(),
    })
    .unwrap();
    let _ = tx.send(join_msg);

    // Spawn relay task: forward room messages to this client
    let relay_handle = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    let expected_psk = std::env::var("OMSPBASE_PSK").ok();
    let mut authenticated = expected_psk.is_none(); // authenticated if no PSK required

    // Process incoming messages
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                let text_str = text.to_string();

                let parsed: Result<SignalingMessage, _> = serde_json::from_str(&text_str);
                match parsed {
                    Ok(SignalingMessage::Auth { token }) => {
                        if let Some(ref psk) = expected_psk {
                            if &token == psk {
                                authenticated = true;
                                tracing::info!("Client authenticated in room {}", room_id);
                                let ack = serde_json::to_string(&SignalingMessage::AuthOk).unwrap();
                                let _ = tx.send(ack);
                            } else {
                                tracing::warn!("Auth failed in room {}", room_id);
                                break; // close connection
                            }
                        }
                    }
                    // Only relay signaling messages (offer, answer, ice) — not auth or system messages
                    Ok(ref msg)
                        if authenticated
                            && matches!(
                                msg,
                                SignalingMessage::Offer { .. }
                                    | SignalingMessage::Answer { .. }
                                    | SignalingMessage::Ice { .. }
                            ) =>
                    {
                        let _ = tx.send(text_str);
                        tracing::debug!(
                            "Relayed {} message in room {}",
                            message_type_name(msg),
                            room_id
                        );
                    }
                    Ok(_) if authenticated => {
                        // Ignore system message types relayed back by the broadcast
                    }
                    Ok(_) => {
                        tracing::warn!("Non-auth message before authentication in room {}", room_id);
                        break;
                    }
                    Err(e) => {
                        tracing::warn!("Invalid signaling JSON in room {}: {}", room_id, e);
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    relay_handle.abort();
    server.leave_room(&room_id, &role);

    // Notify peers about disconnection
    let leave_msg = serde_json::to_string(&SignalingMessage::PeerLeft {
        role: role.clone(),
        id: room_id.clone(),
    })
    .unwrap();
    let _ = tx.send(leave_msg);

    tracing::info!(
        "Client disconnected from room {} (role: {})",
        room_id,
        role
    );
}

fn message_type_name(msg: &SignalingMessage) -> &'static str {
    match msg {
        SignalingMessage::Offer { .. } => "offer",
        SignalingMessage::Answer { .. } => "answer",
        SignalingMessage::Ice { .. } => "ice",
        _ => "other",
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_auth_message() {
        let json = r#"{"type":"auth","token":"test123"}"#;
        let msg: SignalingMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, SignalingMessage::Auth { .. }));
    }

    #[test]
    fn parse_offer_message() {
        let json = r#"{"type":"offer","sdp":"v=0\r\no=- ..."}"#;
        let msg: SignalingMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, SignalingMessage::Offer { .. }));
    }

    #[test]
    fn parse_answer_message() {
        let json = r#"{"type":"answer","sdp":"v=0\r\no=- ..."}"#;
        let msg: SignalingMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, SignalingMessage::Answer { .. }));
    }

    #[test]
    fn parse_ice_message() {
        let json =
            r#"{"type":"ice","candidate":"candidate:...","sdpMid":"0","sdpMLineIndex":0}"#;
        let msg: SignalingMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, SignalingMessage::Ice { .. }));
    }

    #[test]
    fn parse_peer_joined_message() {
        let json = r#"{"type":"peer_joined","role":"host","id":"room-a"}"#;
        let msg: SignalingMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, SignalingMessage::PeerJoined { .. }));
    }

    #[test]
    fn signaling_server_new_creates_empty_rooms() {
        let server = SignalingServer::new();
        assert!(server.active_hosts().is_empty());
        assert!(server.active_remotes().is_empty());
    }

    #[test]
    fn join_and_leave_host() {
        let server = SignalingServer::new();
        server.join_room("room-a", "host");
        assert_eq!(server.active_hosts().len(), 1);
        assert!(server.active_hosts()[0].contains("room-a"));
        server.leave_room("room-a", "host");
        assert!(server.active_hosts().is_empty());
        assert!(server.active_remotes().is_empty());
    }

    #[test]
    fn multiple_remotes_in_room() {
        let server = SignalingServer::new();
        server.join_room("room-a", "remote");
        server.join_room("room-a", "remote");
        server.join_room("room-a", "remote");
        assert!(server.active_remotes()[0].contains("remotes=3"));
        server.leave_room("room-a", "remote");
        assert!(server.active_remotes()[0].contains("remotes=2"));
    }
}
