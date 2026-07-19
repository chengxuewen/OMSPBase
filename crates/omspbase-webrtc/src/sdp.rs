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
#[serde(rename_all = "camelCase")]
pub struct SessionDescription {
    #[serde(rename = "type")]
    pub sdp_type: SdpType,
    pub sdp: String,
}

impl SessionDescription {
    /// Create from type+body (webrtc-kit compatible format).
    pub fn new(sdp_type: SdpType, sdp: String) -> Self {
        Self { sdp_type, sdp }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_each_type() {
        assert_eq!(SdpType::Offer.to_string(), "offer");
        assert_eq!(SdpType::PrAnswer.to_string(), "pranswer");
        assert_eq!(SdpType::Answer.to_string(), "answer");
        assert_eq!(SdpType::Rollback.to_string(), "rollback");
    }

    #[test]
    fn session_description_new() {
        let sd = SessionDescription::new(SdpType::Offer, "v=0".into());
        assert_eq!(sd.sdp_type, SdpType::Offer);
        assert_eq!(sd.sdp, "v=0");
    }

    #[test]
    fn serde_camel_case_roundtrip() {
        let sd = SessionDescription {
            sdp_type: SdpType::Answer,
            sdp: "v=0\r\n".into(),
        };
        let json = serde_json::to_string(&sd).unwrap();
        // camelCase: {"type":"answer","sdp":"v=0\r\n"}
        assert!(json.contains("\"type\":\"answer\""));
        let parsed: SessionDescription = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.sdp_type, SdpType::Answer);
        assert_eq!(parsed.sdp, "v=0\r\n");
    }
}
