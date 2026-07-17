//! WebSocket signaling client — connects to omspbase-server /ws endpoint.
//!
//! Handles: PSK auth → RoomJoin → RoomJoined → SDP/ICE relay loop.

use futures_util::{SinkExt, StreamExt};
use omspbase_core::error::CoreError;
use omspbase_core::protocol::{PeerRole, SignalingMessage};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

/// Manages the signaling WebSocket lifecycle: auth, room join, message relay.
pub struct SignalingClient {
    server_url: String,
    psk: String,
    room_id: String,
}

impl SignalingClient {
    /// Create a new signaling client.
    ///
    /// `server_url` is the base URL (e.g., `ws://server.local:9800`).
    /// The `/ws` path is appended automatically.
    pub fn new(server_url: &str, psk: &str, room_id: &str) -> Self {
        Self {
            server_url: server_url.to_string(),
            psk: psk.to_string(),
            room_id: room_id.to_string(),
        }
    }

    /// Connect to the signaling server, authenticate, join a room,
    /// and enter the SDP/ICE relay loop. Blocks until disconnect.
    pub async fn connect(&self) -> Result<(), CoreError> {
        let ws_url = format!("{}/ws", self.server_url);

        let (mut ws, _) = connect_async(&ws_url)
            .await
            .map_err(|e| CoreError::WebSocketDisconnect(format!("connect failed: {e}")))?;

        // Step 1: Send PSK for authentication
        ws.send(Message::Text(self.psk.clone().into()))
            .await
            .map_err(|e| CoreError::WebSocketDisconnect(format!("send PSK: {e}")))?;

        // Step 2: Wait for auth acknowledgment
        match ws.next().await {
            Some(Ok(Message::Text(text))) => {
                let msg: SignalingMessage = serde_json::from_str(&text)
                    .map_err(|e| CoreError::WebSocketDisconnect(format!("parse auth response: {e}")))?;
                match msg {
                    SignalingMessage::Error { code: 0, .. } => {
                        tracing::info!("Signaling: authenticated");
                    }
                    SignalingMessage::Error { code, message } => {
                        tracing::error!("Auth failed [{code}]: {message}");
                        return Err(CoreError::PskAuthFailed);
                    }
                    _ => {
                        return Err(CoreError::WebSocketDisconnect(
                            "unexpected auth response".into(),
                        ));
                    }
                }
            }
            Some(Ok(_)) => {
                return Err(CoreError::WebSocketDisconnect("non-text auth response".into()));
            }
            Some(Err(e)) => {
                return Err(CoreError::WebSocketDisconnect(format!("auth read error: {e}")));
            }
            None => {
                return Err(CoreError::WebSocketDisconnect(
                    "connection closed during auth".into(),
                ));
            }
        }

        // Step 3: Send RoomJoin
        let join = SignalingMessage::RoomJoin {
            room_id: self.room_id.clone(),
            peer_role: PeerRole::Remote,
        };
        let join_json = serde_json::to_string(&join)
            .map_err(|e| CoreError::ConfigParse(format!("serialize RoomJoin: {e}")))?;
        ws.send(Message::Text(join_json.into()))
            .await
            .map_err(|e| CoreError::WebSocketDisconnect(format!("send RoomJoin: {e}")))?;

        // Step 4: Wait for RoomJoined
        let _peer_id = match ws.next().await {
            Some(Ok(Message::Text(text))) => {
                let msg: SignalingMessage = serde_json::from_str(&text)
                    .map_err(|e| CoreError::WebSocketDisconnect(format!("parse room response: {e}")))?;
                match msg {
                    SignalingMessage::RoomJoined { room_id, peer_id } => {
                        tracing::info!("Signaling: joined room {room_id} as {peer_id}");
                        peer_id
                    }
                    SignalingMessage::Error { code, message } => {
                        tracing::error!("Room join error [{code}]: {message}");
                        return Err(CoreError::WebSocketDisconnect(format!(
                            "room join rejected [{code}]: {message}"
                        )));
                    }
                    _ => {
                        return Err(CoreError::WebSocketDisconnect(
                            "unexpected RoomJoined response".into(),
                        ));
                    }
                }
            }
            Some(Ok(_)) => {
                return Err(CoreError::WebSocketDisconnect("non-text room response".into()));
            }
            Some(Err(e)) => {
                return Err(CoreError::WebSocketDisconnect(format!("room read error: {e}")));
            }
            None => {
                return Err(CoreError::WebSocketDisconnect(
                    "connection closed during room join".into(),
                ));
            }
        };

        // Step 5: SDP/ICE relay loop
        // ponytail: log messages for now; real SDP/ICE handler deferred to WebRTC integration
        while let Some(msg) = ws.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    match serde_json::from_str::<SignalingMessage>(&text) {
                        Ok(SignalingMessage::Sdp { sdp: _, target, .. }) => {
                            tracing::debug!("Signaling: received SDP from {:?}", target);
                        }
                        Ok(SignalingMessage::IceCandidate { candidate: _, .. }) => {
                            tracing::debug!("Signaling: received ICE candidate");
                        }
                        Ok(SignalingMessage::RoomLeave { peer_id, .. }) => {
                            tracing::info!("Signaling: peer {peer_id} left room");
                        }
                        Ok(_) => {}
                        Err(e) => {
                            tracing::warn!("Signaling: parse error: {e}");
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("Signaling: connection closed");
                    break;
                }
                Ok(Message::Ping(data)) => {
                    let _ = ws.send(Message::Pong(data)).await;
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("Signaling: WebSocket error: {e}");
                    break;
                }
            }
        }

        Ok(())
    }
}
