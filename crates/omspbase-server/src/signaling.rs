use crate::room::RoomManager;
use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use futures_util::{SinkExt, StreamExt};
use omspbase_core::auth::SimplePskAuth;
use omspbase_core::error::CoreError;
use omspbase_core::protocol::SignalingMessage;
use std::sync::Arc;
use tokio::sync::broadcast;

struct RoomChannel {
    tx: broadcast::Sender<String>,
}

impl RoomChannel {
    fn new() -> Self {
        let (tx, _) = broadcast::channel::<String>(64);
        Self { tx }
    }
}

#[derive(Clone)]
pub struct SignalingServer {
    channels: Arc<dashmap::DashMap<String, RoomChannel>>,
    pub room_manager: RoomManager,
}

impl SignalingServer {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(dashmap::DashMap::new()),
            room_manager: RoomManager::new(),
        }
    }

    fn get_or_create_channel(&self, room_id: &str) -> broadcast::Sender<String> {
        self.channels
            .entry(room_id.to_string())
            .or_insert_with(RoomChannel::new)
            .tx
            .clone()
    }
}

pub fn signaling_router(server: SignalingServer) -> Router {
    Router::new()
        .route("/ws", get(ws_handler))
        .with_state(server)
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(server): State<SignalingServer>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, server))
}

async fn handle_socket(socket: WebSocket, server: SignalingServer) {
    let (mut sender, mut receiver) = socket.split();
    let peer_id = uuid::Uuid::new_v4().to_string();
    tracing::info!("New connection: peer={}", peer_id);

    // PSK auth — from env var for Phase 1
    let psk = std::env::var("OMSPBASE_PSK").ok();
    let auth = psk.as_ref().map(|k| SimplePskAuth::new(k.as_bytes()));
    let mut authenticated = auth.is_none();

    // Phase 1: Authentication
    if !authenticated {
        match receiver.next().await {
            Some(Ok(Message::Text(text))) => {
                // ponytail: simple token-as-first-message; upgrade to challenge-response later
                if let Some(ref a) = auth {
                    if a.sign(peer_id.as_bytes()) == a.sign(text.as_bytes())
                        || text == psk.as_deref().unwrap_or("")
                    {
                        authenticated = true;
                        tracing::info!("Peer {} authenticated", peer_id);
                    }
                }
                if !authenticated {
                    let error = SignalingMessage::Error {
                        code: 4003,
                        message: "PSK authentication failed".into(),
                    };
                    let _ = sender
                        .send(Message::Text(serde_json::to_string(&error).unwrap().into()))
                        .await;
                    return;
                }
            }
            _ => {
                let error = SignalingMessage::Error {
                    code: 4003,
                    message: "Authentication required".into(),
                };
                let _ = sender
                    .send(Message::Text(serde_json::to_string(&error).unwrap().into()))
                    .await;
                return;
            }
        }
        let ack = SignalingMessage::Error {
            code: 0,
            message: "authenticated".into(),
        };
        let _ = sender
            .send(Message::Text(serde_json::to_string(&ack).unwrap().into()))
            .await;
    }

    // Phase 2: RoomJoin
    let (room_id, role) = loop {
        match receiver.next().await {
            Some(Ok(Message::Text(text))) => {
                let text_str = text.to_string();
                if let Ok(SignalingMessage::RoomJoin { room_id, peer_role }) =
                    serde_json::from_str(&text_str)
                {
                    break (room_id, peer_role);
                }
            }
            Some(Ok(Message::Close(_))) | None => return,
            _ => continue,
        }
    };

    // Join the room
    match server.room_manager.join_room(&room_id, &peer_id, &role) {
        Ok(()) => {}
        Err(CoreError::RoomFull) => {
            let error = SignalingMessage::Error {
                code: 4002,
                message: "Room is full".into(),
            };
            let _ = sender
                .send(Message::Text(serde_json::to_string(&error).unwrap().into()))
                .await;
            return;
        }
        Err(e) => {
            tracing::error!("Room join error: {}", e);
            let error = SignalingMessage::Error {
                code: 4001,
                message: format!("Failed to join room: {}", e),
            };
            let _ = sender
                .send(Message::Text(serde_json::to_string(&error).unwrap().into()))
                .await;
            return;
        }
    }

    // Send RoomJoined ack
    let ack = SignalingMessage::RoomJoined {
        room_id: room_id.clone(),
        peer_id: peer_id.clone(),
    };
    let _ = sender
        .send(Message::Text(serde_json::to_string(&ack).unwrap().into()))
        .await;

    let tx = server.get_or_create_channel(&room_id);
    let mut rx = tx.subscribe();

    // Phase 3: Message relay
    let relay_peer_id = peer_id.clone();
    let relay_room = room_id.clone();

    // Spawn: broadcast → this peer's sender
    let mut relay_sender = sender;
    let relay_handle = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if relay_sender
                .send(Message::Text(msg.into()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Forward: this peer's receiver → broadcast
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                let text_str = text.to_string();
                if let Ok(sig_msg) = serde_json::from_str::<SignalingMessage>(&text_str) {
                    if matches!(
                        sig_msg,
                        SignalingMessage::Sdp { .. } | SignalingMessage::IceCandidate { .. }
                            | SignalingMessage::Frame { .. }
                    ) {
                        let _ = tx.send(text_str);
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    relay_handle.abort();
    server.room_manager.leave_room(&relay_room, &relay_peer_id);

    let leave_msg = SignalingMessage::RoomLeave {
        room_id: relay_room.clone(),
        peer_id: relay_peer_id.clone(),
    };
    let _ = tx.send(serde_json::to_string(&leave_msg).unwrap());

    tracing::info!(
        "Peer {} disconnected from room {}",
        relay_peer_id,
        relay_room
    );
}
