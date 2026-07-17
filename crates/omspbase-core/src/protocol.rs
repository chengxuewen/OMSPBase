//! Signaling protocol message types.
//!
//! All signaling messages flow through the Server's WebSocket /ws endpoint
//! as JSON. Server relays messages between Host and Remote without modification
//! (except for room management messages).

use serde::{Deserialize, Serialize};

/// A signaling message exchanged via WebSocket.
///
/// # Flow
/// ```text
/// Host ──WS──▶ Server ──WS──▶ Remote
/// Remote ──WS──▶ Server ──WS──▶ Host
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignalingMessage {
    /// Request to join a room. Sent by Host or Remote to Server.
    RoomJoin {
        room_id: String,
        peer_role: PeerRole,
    },

    /// Room join acknowledged by Server.
    RoomJoined {
        room_id: String,
        peer_id: String,
    },

    /// A peer has left the room. Broadcast by Server.
    RoomLeave {
        room_id: String,
        peer_id: String,
    },

    /// SDP offer/answer relayed through Server.
    Sdp {
        room_id: String,
        target: Option<String>,
        sdp: String,
    },

    /// ICE candidate relayed through Server.
    IceCandidate {
        room_id: String,
        target: Option<String>,
        candidate: String,
        sdp_mid: Option<String>,
        sdp_mline_index: Option<u16>,
    },

    /// Error response from Server.
    Error {
        code: u16,
        message: String,
    },
}

/// Role of a peer in a room.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PeerRole {
    Host,
    Remote,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_room_join() {
        let msg = SignalingMessage::RoomJoin {
            room_id: "room-1".into(),
            peer_role: PeerRole::Host,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"room_join""#));
        assert!(json.contains(r#""room_id":"room-1""#));
        assert!(json.contains(r#""peer_role":"host""#));
    }

    #[test]
    fn serialize_error() {
        let msg = SignalingMessage::Error {
            code: 4003,
            message: "PSK authentication failed".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"error""#));
        assert!(json.contains("4003"));
    }

    #[test]
    fn roundtrip_ice_candidate() {
        let msg = SignalingMessage::IceCandidate {
            room_id: "r1".into(),
            target: None,
            candidate: "candidate:1 1 UDP 2130706431 10.0.0.1 8000 typ host".into(),
            sdp_mid: Some("0".into()),
            sdp_mline_index: Some(0),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: SignalingMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, SignalingMessage::IceCandidate { .. }));
    }
}
