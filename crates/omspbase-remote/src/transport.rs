//! WebRTC transport — PeerConnection factory for remote pull side.
//!
//! Feature-gated behind `cfg(feature = "webrtc")`. In default builds,
//! a stub is provided that logs the server address.

use omspbase_core::error::CoreError;

/// WebRTC transport placeholder.
///
/// In production (with `webrtc` feature), manages a PeerConnection
/// that pulls remote tracks from the server relay.
#[cfg(feature = "webrtc")]
pub struct Transport {
    _server_addr: String,
}

#[cfg(feature = "webrtc")]
impl Transport {
    pub fn new(server_addr: &str) -> Self {
        tracing::info!("Remote transport targeting {}", server_addr);
        Self {
            _server_addr: server_addr.to_string(),
        }
    }

    pub async fn connect(&self) -> Result<(), CoreError> {
        // ponytail: stub — real PeerConnection connect deferred to full WebRTC integration
        tracing::info!("WebRTC transport connect stub");
        Ok(())
    }
}

/// Stub when `webrtc` feature is disabled.
#[cfg(not(feature = "webrtc"))]
pub struct Transport;

#[cfg(not(feature = "webrtc"))]
impl Transport {
    pub fn new(server_addr: &str) -> Self {
        tracing::info!("Remote transport to {} (stub)", server_addr);
        Self
    }

    pub async fn connect(&self) -> Result<(), CoreError> {
        Ok(())
    }
}
