//! W3C RTCRtpSender and RTCRtpReceiver types (D146).
//!
//! RTCRtpSender wraps a TrackSender with sender metadata.
//! RTCRtpReceiver wraps a TrackReceiver with receiver metadata.

use crate::track::{TrackKind, TrackRef};

/// W3C RTCRtpSender — wraps a TrackRef::Sender with sender metadata (D146).
#[derive(Debug, Clone)]
pub struct RTCRtpSender {
    pub track: TrackRef,
    pub track_id: String,
    pub kind: TrackKind,
}

impl RTCRtpSender {
    pub fn new(track: TrackRef) -> Self {
        let track_id = track.id().to_string();
        let kind = track.kind();
        Self {
            track,
            track_id,
            kind,
        }
    }
}

/// W3C RTCRtpReceiver — wraps a TrackRef::Receiver with receiver metadata (D146).
#[derive(Debug, Clone)]
pub struct RTCRtpReceiver {
    pub track: TrackRef,
    pub track_id: String,
    pub kind: TrackKind,
}

impl RTCRtpReceiver {
    pub fn new(track: TrackRef) -> Self {
        let track_id = track.id().to_string();
        let kind = track.kind();
        Self {
            track,
            track_id,
            kind,
        }
    }
}
