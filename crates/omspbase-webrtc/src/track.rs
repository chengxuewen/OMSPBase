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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_fields() {
        let tr = TrackReceiver::new("track-1".into(), "video".into());
        assert_eq!(tr.id, "track-1");
        assert_eq!(tr.kind, "video");
    }

    #[test]
    fn clone_preserves_fields() {
        let tr = TrackReceiver::new("t2".into(), "audio".into());
        let tr2 = tr.clone();
        assert_eq!(tr2.id, "t2");
        assert_eq!(tr2.kind, "audio");
    }
}
