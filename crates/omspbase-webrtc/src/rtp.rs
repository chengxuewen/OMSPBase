//! W3C RTCRtpSender and RTCRtpReceiver types (D146).
//!
//! RtpSender wraps a TrackSender with sender metadata.
//! RtpReceiver wraps a TrackReceiver with receiver metadata.

use crate::track::{TrackKind, TrackRef};

/// W3C RTCRtpSender — wraps a TrackRef::Sender with sender metadata (D146).
#[derive(Debug, Clone)]
pub struct RtpSender {
    pub track: TrackRef,
    pub track_id: String,
    pub kind: TrackKind,
}

impl RtpSender {
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
pub struct RtpReceiver {
    pub track: TrackRef,
    pub track_id: String,
    pub kind: TrackKind,
}

impl RtpReceiver {
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
