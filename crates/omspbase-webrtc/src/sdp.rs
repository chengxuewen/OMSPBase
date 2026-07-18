//! SDP session description types.
//!
//! Follows the webrtc-kit convention: SDP strings use "\n" as separator
//! between type and body (e.g., "offer\nv=0\r\n...").

use serde::{Deserialize, Serialize};
use std::fmt;

/// SDP type as per W3C RTCSdpType.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SdpType {
    Offer,
    PrAnswer,
    Answer,
    Rollback,
}

impl fmt::Display for SdpType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            SdpType::Offer => "offer",
            SdpType::PrAnswer => "pranswer",
            SdpType::Answer => "answer",
            SdpType::Rollback => "rollback",
        };
        write!(f, "{s}")
    }
}

/// A parsed session description with type and SDP body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDescription {
    pub sdp_type: SdpType,
    pub sdp: String,
}

impl SessionDescription {
    /// Create from type+body (webrtc-kit compatible format).
    pub fn new(sdp_type: SdpType, sdp: String) -> Self {
        Self { sdp_type, sdp }
    }
}
