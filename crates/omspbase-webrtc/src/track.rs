//! Video track wrapper around webrtc-sys.

/// Receiver-side video track.
/// Receives frames from a remote PeerConnection via on_track callback.
#[derive(Debug, Clone)]
pub struct TrackReceiver {
    pub id: String,
    pub kind: String,
}

impl TrackReceiver {
    pub fn new(id: String, kind: String) -> Self {
        Self { id, kind }
    }
}
