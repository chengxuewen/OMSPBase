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

    #[test]
    fn roundtrip_room_joined() {
        let msg = SignalingMessage::RoomJoined {
            room_id: "room-42".into(),
            peer_id: "peer-7".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"room_joined""#));
        let parsed: SignalingMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            SignalingMessage::RoomJoined { room_id, peer_id } => {
                assert_eq!(room_id, "room-42");
                assert_eq!(peer_id, "peer-7");
            }
            _ => panic!("expected RoomJoined"),
        }
    }

    #[test]
    fn roundtrip_room_leave() {
        let msg = SignalingMessage::RoomLeave {
            room_id: "room-99".into(),
            peer_id: "peer-3".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"room_leave""#));
        let parsed: SignalingMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            SignalingMessage::RoomLeave { room_id, peer_id } => {
                assert_eq!(room_id, "room-99");
                assert_eq!(peer_id, "peer-3");
            }
            _ => panic!("expected RoomLeave"),
        }
    }

    #[test]
    fn roundtrip_sdp() {
        let msg = SignalingMessage::Sdp {
            room_id: "room-1".into(),
            target: Some("peer-a".into()),
            sdp: "v=0\r\no=- 1 2 IN IP4 127.0.0.1\r\ns=-".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"sdp""#));
        let parsed: SignalingMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            SignalingMessage::Sdp { room_id, target, sdp } => {
                assert_eq!(room_id, "room-1");
                assert_eq!(target.as_deref(), Some("peer-a"));
                assert!(sdp.starts_with("v=0"));
            }
            _ => panic!("expected Sdp"),
        }
    }

    #[test]
    fn roundtrip_sdp_without_target() {
        let msg = SignalingMessage::Sdp {
            room_id: "room-x".into(),
            target: None,
            sdp: "v=0".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: SignalingMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            SignalingMessage::Sdp { target, .. } => {
                assert!(target.is_none());
            }
            _ => panic!("expected Sdp"),
        }
    }

    #[test]
    fn peer_role_host_serde() {
        let json = serde_json::to_string(&PeerRole::Host).unwrap();
        assert_eq!(json, r#""host""#);
        let parsed: PeerRole = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, PeerRole::Host);
    }

    #[test]
    fn peer_role_remote_serde() {
        let json = serde_json::to_string(&PeerRole::Remote).unwrap();
        assert_eq!(json, r#""remote""#);
        let parsed: PeerRole = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, PeerRole::Remote);
    }

    #[test]
    fn deserialize_unknown_type() {
        let json = r#"{"type":"unknown_kind","room_id":"x"}"#;
        let result: Result<SignalingMessage, _> = serde_json::from_str(json);
        assert!(result.is_err(), "unknown type should fail deserialization");
    }

    #[test]
    fn deserialize_missing_required_field() {
        let json = r#"{"type":"error","message":"oops"}"#;
        // Error variant requires both code and message
        let result: Result<SignalingMessage, _> = serde_json::from_str(json);
        assert!(result.is_err(), "missing 'code' field should fail");
    }

    #[test]
    fn deserialize_bad_peer_role() {
        let json = r#""invalid_role""#;
        let result: Result<PeerRole, _> = serde_json::from_str(json);
        assert!(result.is_err(), "invalid role should fail deserialization");
    }
