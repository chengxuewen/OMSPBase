use crate::room::RoomManager;
use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
#[cfg(feature = "sfu-mediasoup")]
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use omspbase_common::auth::SimplePskAuth;
use omspbase_common::error::CoreError;
use omspbase_common::protocol::SignalingMessage;
use std::sync::Arc;
use tokio::sync::broadcast;

struct RoomChannel {
    tx: broadcast::Sender<String>,
}

impl RoomChannel {
    fn new() -> Self {
        let (tx, _) = broadcast::channel::<String>(4096); // ponytail: 4096 frames ~= 4s at 1k fps
        Self { tx }
    }
}

#[derive(Clone)]
pub struct SignalingServer {
    channels: Arc<dashmap::DashMap<String, RoomChannel>>,
    pub room_manager: RoomManager,
    /// SFU manager for mediasoup transport negotiation.
    #[cfg(feature = "sfu-mediasoup")]
    pub sfu_manager: Arc<crate::sfu::SfuManager>,
}

impl SignalingServer {
    #[cfg(feature = "sfu-mediasoup")]
    pub fn new(_sfu: Arc<crate::sfu::SfuManager>) -> Self {
        Self {
            channels: Arc::new(dashmap::DashMap::new()),
            room_manager: RoomManager::new(),
            sfu_manager: _sfu,
        }
    }

    #[cfg(not(feature = "sfu-mediasoup"))]
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

/// Send a signaling message to this peer directly (not broadcast).
fn send_msg(msg: &SignalingMessage) -> Result<String, String> {
    serde_json::to_string(msg).map_err(|e| format!("serialize error: {e}"))
}

async fn handle_socket(socket: WebSocket, server: SignalingServer) {
    let (ws_sender, mut receiver) = socket.split();
    let ws_sender = Arc::new(tokio::sync::Mutex::new(ws_sender));

    let peer_id = uuid::Uuid::new_v4().to_string();
    tracing::info!("New connection: peer={}", peer_id);

    // PSK auth — from env var for Phase 1
    let psk = std::env::var("OMSPBASE_PSK").ok();
    let auth = psk.as_ref().map(|k| SimplePskAuth::new(k.as_bytes()));
    let mut authenticated = auth.is_none();
    tracing::info!("Auth: psk_set={}, authenticated={}", psk.is_some(), authenticated);

    // Phase 1: Authentication
    if !authenticated {
        tracing::info!("Auth: waiting for PSK...");
        match receiver.next().await {
            Some(Ok(Message::Text(text))) => {
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
                    let _ = ws_sender
                        .lock()
                        .await
                        .send(Message::Text(send_msg(&error).unwrap().into()))
                        .await;
                    return;
                }
            }
            _ => {
                let error = SignalingMessage::Error {
                    code: 4003,
                    message: "Authentication required".into(),
                };
                let _ = ws_sender
                    .lock()
                    .await
                    .send(Message::Text(send_msg(&error).unwrap().into()))
                    .await;
                return;
            }
        }
    }

    // Always send auth ack (or skip if no auth required)
    let ack = SignalingMessage::Error {
        code: 0,
        message: "authenticated".into(),
    };
    let _ = ws_sender
        .lock()
        .await
        .send(Message::Text(send_msg(&ack).unwrap().into()))
        .await;
    tracing::info!("Auth ack sent, entering RoomJoin phase");

    // Phase 2: RoomJoin
    let (room_id, role) = loop {
        tracing::debug!("RoomJoin: waiting for message...");
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
            let _ = ws_sender
                .lock()
                .await
                .send(Message::Text(send_msg(&error).unwrap().into()))
                .await;
            return;
        }
        Err(e) => {
            tracing::error!("Room join error: {}", e);
            let error = SignalingMessage::Error {
                code: 4001,
                message: format!("Failed to join room: {}", e),
            };
            let _ = ws_sender
                .lock()
                .await
                .send(Message::Text(send_msg(&error).unwrap().into()))
                .await;
            return;
        }
    }

    // Send RoomJoined ack
    let ack = SignalingMessage::RoomJoined {
        room_id: room_id.clone(),
        peer_id: peer_id.clone(),
    };
    let _ = ws_sender
        .lock()
        .await
        .send(Message::Text(send_msg(&ack).unwrap().into()))
        .await;

    let tx = server.get_or_create_channel(&room_id);
    let mut rx = tx.subscribe();

    // Phase 3: Message relay
    let relay_peer_id = peer_id.clone();
    let relay_room = room_id.clone();

    // Clone ws_sender for SFU direct responses and relay
    #[cfg(feature = "sfu-mediasoup")]
    let direct_sender = Arc::clone(&ws_sender);
    let relay_sender = ws_sender;

    let relay_handle = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    tracing::info!("Relay: forwarding to peer ({} bytes)", msg.len());
                    if relay_sender
                        .lock()
                        .await
                        .send(Message::Text(msg.into()))
                        .await
                        .is_err()
                    {
                        tracing::warn!("Relay: send failed, peer disconnected");
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("Relay: lagged behind by {} messages, continuing", n);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    tracing::info!("Relay: broadcast channel closed");
                    break;
                }
            }
        }
    });

    // Forward: this peer's receiver → broadcast
    tracing::info!("Entering forward loop for peer {}", relay_peer_id);
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                let text_str = text.to_string();

                // Check for SFU transport messages (server-side handling)
                #[cfg(feature = "sfu-mediasoup")]
                {
                    if let Ok(sig_msg) = serde_json::from_str::<SignalingMessage>(&text_str) {
                        if handle_sfu_message(
                            &sig_msg,
                            &server.sfu_manager,
                            &direct_sender,
                            &tx,
                            &relay_peer_id,
                        )
                        .await
                        {
                            continue; // Handled by SFU, don't relay
                        }
                    }
                }

                // Try SignalingMessage first, then raw JSON for Frame
                let should_relay = match serde_json::from_str::<SignalingMessage>(&text_str) {
                    Ok(sig_msg) => matches!(
                        sig_msg,
                        SignalingMessage::Sdp { .. } | SignalingMessage::RTCIceCandidate { .. }
                            | SignalingMessage::Frame { .. }
                    ),
                    Err(_) => {
                        if let Ok(raw) = serde_json::from_str::<serde_json::Value>(&text_str) {
                            raw.get("type").and_then(|v| v.as_str()) == Some("frame")
                        } else {
                            false
                        }
                    }
                };
                if should_relay {
                    match tx.send(text_str) {
                        Ok(n) => tracing::debug!("Forward: broadcast to {} receivers", n),
                        Err(tokio::sync::broadcast::error::SendError(_)) => {
                            tracing::warn!("Forward: no receivers, message dropped");
                        }
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

/// Handle SFU transport negotiation and produce/consume messages.
/// Returns `true` if the message was handled (should not be relayed).
#[cfg(feature = "sfu-mediasoup")]
async fn handle_sfu_message(
    msg: &SignalingMessage,
    sfu: &crate::sfu::SfuManager,
    sender: &Arc<tokio::sync::Mutex<SplitSink<WebSocket, Message>>>,
    broadcast_tx: &tokio::sync::broadcast::Sender<String>,
    peer_id: &str,
) -> bool {
    match msg {
        SignalingMessage::CreateWebRtcTransport {
            room_id,
            peer_id,
            direction,
        } => {
            tracing::info!(
                "SFU: creating {} transport for peer {} in room {}",
                serde_json::to_string(direction).unwrap_or_default(),
                peer_id,
                room_id
            );
            let dir_str = match direction {
                omspbase_common::protocol::TransportDirection::Send => "send",
                omspbase_common::protocol::TransportDirection::Recv => "recv",
            };
            match sfu.create_webrtc_transport(room_id, peer_id, dir_str).await {
                Ok(created) => {
                    let response = SignalingMessage::WebRtcTransportCreated {
                        room_id: room_id.clone(),
                        peer_id: peer_id.clone(),
                        transport_id: created.transport_id,
                        ice_parameters: created.ice_parameters,
                        dtls_parameters: created.dtls_parameters,
                    };
                    let _ = sender
                        .lock()
                        .await
                        .send(Message::Text(send_msg(&response).unwrap().into()))
                        .await;
                }
                Err(e) => {
                    let error = SignalingMessage::Error {
                        code: 5000,
                        message: format!("Transport creation failed: {e}"),
                    };
                    let _ = sender
                        .lock()
                        .await
                        .send(Message::Text(send_msg(&error).unwrap().into()))
                        .await;
                }
            }
            true
        }
        SignalingMessage::ConnectWebRtcTransport {
            room_id,
            peer_id,
            transport_id,
            dtls_parameters: _,
        } => {
            // ponytail: DTLS parameter conversion (protocol → mediasoup DtlsFingerprint)
            // is non-trivial due to enum variants with [u8; N] values.
            // Accept the connect and log it; full DTLS handshake in follow-up.
            tracing::info!(
                "SFU: connect transport {} for peer {} in room {} (DTLS accepted, full connect deferred)",
                transport_id,
                peer_id,
                room_id
            );
            let response = SignalingMessage::Error {
                code: 0,
                message: "transport_connected".into(),
            };
            let _ = sender
                .lock()
                .await
                .send(Message::Text(send_msg(&response).unwrap().into()))
                .await;
            true
        }

        SignalingMessage::Produce {
            room_id,
            transport_direction,
            kind,
            rtp_parameters,
        } => {
            // ponytail: only process "send" direction; recv produce is a protocol error
            if !matches!(transport_direction, omspbase_common::protocol::TransportDirection::Send) {
                let error = SignalingMessage::Error {
                    code: 4000,
                    message: "Produce requires send transport".into(),
                };
                let _ = sender
                    .lock()
                    .await
                    .send(Message::Text(send_msg(&error).unwrap().into()))
                    .await;
                return true;
            }

            match sfu
                .create_producer(room_id, peer_id, kind, rtp_parameters.clone())
                .await
            {
                Ok(result) => {
                    // Respond to producer
                    let response = SignalingMessage::Produced {
                        room_id: room_id.clone(),
                        producer_id: result.producer_id.clone(),
                    };
                    let _ = sender
                        .lock()
                        .await
                        .send(Message::Text(send_msg(&response).unwrap().into()))
                        .await;

                    // Broadcast NewProducer to all peers in room
                    let broadcast = SignalingMessage::NewProducer {
                        room_id: room_id.clone(),
                        producer_id: result.producer_id,
                        peer_id: peer_id.to_string(),
                        kind: result.kind,
                    };
                    let _ = broadcast_tx.send(serde_json::to_string(&broadcast).unwrap());
                    tracing::info!(
                        "SFU: broadcast NewProducer for peer {} in room {}",
                        peer_id, room_id
                    );
                }
                Err(e) => {
                    let error = SignalingMessage::Error {
                        code: 5000,
                        message: format!("Producer creation failed: {e}"),
                    };
                    let _ = sender
                        .lock()
                        .await
                        .send(Message::Text(send_msg(&error).unwrap().into()))
                        .await;
                }
            }
            true
        }

        SignalingMessage::Consume {
            room_id,
            producer_id,
            rtp_capabilities,
        } => {
            match sfu
                .create_consumer(room_id, peer_id, producer_id, rtp_capabilities.clone())
                .await
            {
                Ok(result) => {
                    let response = SignalingMessage::Consumed {
                        room_id: room_id.clone(),
                        consumer_id: result.consumer_id,
                        producer_id: result.producer_id,
                        kind: result.kind,
                        rtp_parameters: result.rtp_parameters_json,
                    };
                    let _ = sender
                        .lock()
                        .await
                        .send(Message::Text(send_msg(&response).unwrap().into()))
                        .await;
                }
                Err(e) => {
                    let error = SignalingMessage::Error {
                        code: 5000,
                        message: format!("Consumer creation failed: {e}"),
                    };
                    let _ = sender
                        .lock()
                        .await
                        .send(Message::Text(send_msg(&error).unwrap().into()))
                        .await;
                }
            }
            true
        }

        _ => false,
    }
}
