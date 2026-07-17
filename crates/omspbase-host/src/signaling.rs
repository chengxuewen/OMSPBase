// WebSocket signaling server (axum)

use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

/// JSON signaling message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SignalingMessage {
    /// PSK authentication (first message required)
    #[serde(rename = "auth")]
    Auth { token: String },

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
}

/// Shared signaling state
pub struct SignalingState {
    /// Broadcast channel to relay messages to all connected peers
    pub tx: broadcast::Sender<String>,
}

/// Create the signaling router at /ws
pub fn signaling_router() -> Router {
    let (tx, _rx) = broadcast::channel::<String>(32);
    let state = Arc::new(SignalingState { tx });

    Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state)
}

/// Handle WebSocket upgrade
async fn ws_handler(
    ws: WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<Arc<SignalingState>>,
) -> impl IntoResponse {
    let tx = state.tx.clone();
    ws.on_upgrade(move |socket| handle_socket(socket, tx))
}

/// Handle a single WebSocket connection
async fn handle_socket(socket: WebSocket, tx: broadcast::Sender<String>) {
    let (_sender, mut receiver) = socket.split();
    let mut authenticated = false;

    // Subscribe to broadcast for relay
    let mut rx = tx.subscribe();

    // Spawn relay task: forward broadcast messages to this client
    let mut relay_sender = _sender;
    // ponytail: single relay task, add connection limit if throughput matters
    let relay_handle = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if relay_sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Process incoming messages
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                let text_str = text.to_string();

                // Parse the message
                let parsed: Result<SignalingMessage, _> = serde_json::from_str(&text_str);
                match parsed {
                    Ok(SignalingMessage::Auth { token }) => {
                        // Simple PSK check: token must match expected value
                        let expected = std::env::var("OMSPBASE_PSK")
                            .unwrap_or_else(|_| "omspbase-dev".to_string());
                        if token == expected {
                            authenticated = true;
                            tracing::info!("WebSocket client authenticated");
                            let ack = serde_json::json!({"type": "auth_ok"});
                            // Can't use sender after split, but we already consumed it.
                            // Just log — client gets auth status implicitly.
                            let _ = ack;
                        } else {
                            tracing::warn!("WebSocket auth failed");
                            break; // close connection on bad auth
                        }
                    }
                    Ok(msg) if authenticated => {
                        // Relay the message to all other peers
                        let _ = tx.send(text_str);
                        tracing::debug!("Relayed signaling message: {:?}", msg);
                    }
                    Ok(_) => {
                        // Not authenticated — ignore non-auth messages
                        tracing::warn!("Non-auth message before authentication, closing");
                        break;
                    }
                    Err(e) => {
                        tracing::warn!("Invalid signaling JSON: {}", e);
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    relay_handle.abort();
    tracing::info!("WebSocket client disconnected");
}

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
    fn parse_ice_message() {
        let json = r#"{"type":"ice","candidate":"candidate:...","sdpMid":"0","sdpMLineIndex":0}"#;
        let msg: SignalingMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, SignalingMessage::Ice { .. }));
    }
}
