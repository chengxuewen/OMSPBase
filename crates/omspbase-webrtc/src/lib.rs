//! omspbase-webrtc — multi-backend W3C WebRTC API wrapper.
//!
//! Provides PeerConnection and DataChannel types through
//! [`RtcEngine::create_factory`] as the unified entry point.
//! Two backends:
//! - `backend-webrtc-rs` feature: real webrtc-rs implementation
//! - default (no feature): stub for compilation without WebRTC

pub mod channel;
pub mod peer;
pub mod sdp;
pub mod track;
pub mod engine;
pub mod rtp;
pub mod rtp_params;
pub mod stats;
pub(crate) mod backend;

pub use channel::*;
pub use peer::*;
pub use sdp::*;
pub use track::*;
pub use engine::*;
pub use rtp::*;
pub use rtp_params::*;
pub use stats::*;

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

#[cfg(feature = "backend-webrtc-rs")]
impl From<webrtc::error::Error> for RtcError {
    fn from(e: webrtc::error::Error) -> Self {
        RtcError::Internal(e.to_string())
    }
}

impl RtcError {
    /// Return a stable, context-free identifier for this error.
    /// Used by future i18n layers to look up locale-specific text.
    pub fn locale_key(&self) -> &'static str {
        match self {
            RtcError::PeerConnection(_) => "RTCPC",
            RtcError::DataChannel(_) => "RTCDC",
            RtcError::Sdp(_) => "RTCSD",
            RtcError::Track(_) => "RTCTK",
            RtcError::Internal(_) => "RTCIN",
        }
    }
}

/// Re-export webrtc-rs for callback types used by consumers.
#[cfg(feature = "backend-webrtc-rs")]
pub use webrtc;
