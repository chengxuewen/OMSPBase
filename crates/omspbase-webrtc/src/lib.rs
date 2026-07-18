//! omspbase-webrtc — thin wrapper around webrtc-rs (pure Rust WebRTC).
//!
//! Provides PeerConnection and DataChannel types with two backends:
//! - `webrtc-backend` feature: real webrtc-rs implementation
//! - default (no feature): stub for compilation without WebRTC

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

#[cfg(feature = "webrtc-backend")]
impl From<webrtc::error::Error> for RtcError {
    fn from(e: webrtc::error::Error) -> Self {
        RtcError::Internal(e.to_string())
    }
}

/// Re-export webrtc-rs for callback types used by consumers.
#[cfg(feature = "webrtc-backend")]
pub use webrtc;
