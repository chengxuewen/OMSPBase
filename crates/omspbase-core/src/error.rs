//! Unified error codes for all OMSPBase components.
//!
//! Error code ranges:
//! - 1xxx — connectivity (WebSocket, ICE, network)
//! - 2xxx — encoding
//! - 3xxx — capture
//! - 4xxx — relay / server
//! - 5xxx — decoding / render
//! - 6xxx — control
//! - 9xxx — system (OOM, config, unknown)

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    // --- 1xxx: Connectivity ---

    /// WebSocket disconnect (Host/Remote ↔ Server signaling)
    #[error("[1001] WebSocket disconnected: {0}")]
    WebSocketDisconnect(String),

    /// ICE connection timeout
    #[error("[1003] ICE connection timeout")]
    IceTimeout,

    /// PeerConnection creation failed
    #[error("[1004] PeerConnection creation failed: {0}")]
    PeerConnectionFailure(String),

    // --- 2xxx: Encoding ---

    /// Encoder initialization failed
    #[error("[2001] Encoder init failed: {0}")]
    EncoderInit(String),

    // --- 3xxx: Capture ---

    /// Capture source not found
    #[error("[3001] Capture source not found: {0}")]
    CaptureSourceNotFound(String),

    /// Capture source disconnected
    #[error("[3002] Capture source disconnected")]
    CaptureDisconnected,

    // --- 4xxx: Relay / Server ---

    /// Relay track binding failed
    #[error("[4001] Relay track bind failed: {0}")]
    RelayTrackBind(String),

    /// Room is full
    #[error("[4002] Room full")]
    RoomFull,

    /// PSK authentication failed
    #[error("[4003] PSK authentication failed")]
    PskAuthFailed,

    // --- 5xxx: Decode / Render ---

    /// Decoder init failed
    #[error("[5001] Decoder init failed: {0}")]
    DecoderInit(String),

    // --- 6xxx: Control ---

    /// HMAC signature verification failed
    #[error("[6001] Control HMAC verification failed")]
    ControlHmacFailed,

    // --- 9xxx: System ---

    /// Out of memory
    #[error("[9001] Out of memory (RSS limit exceeded)")]
    OutOfMemory,

    /// Config parse error
    #[error("[9002] Config parse error: {0}")]
    ConfigParse(String),

    /// Unknown system error
    #[error("[9003] Unknown system error: {0}")]
    Unknown(String),
}

/// Recoverability classification.
impl CoreError {
    /// Returns true if the error is transient and retry may succeed.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            CoreError::WebSocketDisconnect(_)
                | CoreError::IceTimeout
                | CoreError::CaptureDisconnected
                | CoreError::RoomFull
        )
    }

    /// Returns true if the error is fatal (component must restart).
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            CoreError::EncoderInit(_)
                | CoreError::CaptureSourceNotFound(_)
                | CoreError::DecoderInit(_)
                | CoreError::OutOfMemory
                | CoreError::ConfigParse(_)
        )
    }
}
