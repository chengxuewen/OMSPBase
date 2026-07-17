//! omspbase-webrtc — thin wrapper around webrtc-sys (livekit/libwebrtc FFI).
//!
//! Pattern: each type holds a `handle` to the webrtc-sys C++ object.
//! All async methods require a tokio runtime.

pub mod channel;
pub mod peer;
pub mod sdp;
pub mod track;

pub use channel::*;
pub use peer::*;
pub use sdp::*;
pub use track::*;

/// Error type for all WebRTC operations.
#[derive(Debug, thiserror::Error)]
pub enum RtcError {
    #[error("PeerConnection error: {0}")]
    PeerConnection(String),
    #[error("DataChannel error: {0}")]
    DataChannel(String),
    #[error("SDP error: {0}")]
    Sdp(String),
    #[error("Track error: {0}")]
    Track(String),
    #[error("Internal error: {0}")]
    Internal(String),
}
