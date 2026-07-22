// ── RTCPeerConnectionFactory ──

use std::collections::HashMap;
use std::sync::Arc;

use crate::backend::ActiveFactory;
use crate::peer_connection::{RTCPeerConnection, RTCConfiguration};
use crate::track::{TrackSender, TrackKind};
use crate::RTCError;

pub struct RTCPeerConnectionFactory {
    pub backend: ActiveFactory,
}

impl RTCPeerConnectionFactory {
    pub fn new() -> Self {
        Self { backend: ActiveFactory::default() }
    }

    pub async fn create_peer_connection(&self, config: RTCConfiguration) -> Result<RTCPeerConnection, RTCError> {
        let pc_backend = self.backend.create_peer_connection(config).await?;
        Ok(RTCPeerConnection {
            backend: pc_backend,
            tracks: Arc::new(std::sync::Mutex::new(HashMap::new())),
            on_track_callback: Arc::new(std::sync::Mutex::new(None)),
        })
    }

    /// Create a video track with a real VideoTrackSource (webrtc-sys only).
    /// For other backends, returns a stub TrackSender.
    pub fn create_video_track(&self, track_id: &str) -> TrackSender {
        #[cfg(feature = "backend-webrtc-sys")]
        {
            let (backend, _media_track) = self.backend.create_video_track();
            TrackSender { id: track_id.to_string(), kind: TrackKind::Video, audio_config: None, backend }
        }
        #[cfg(not(feature = "backend-webrtc-sys"))]
        {
            TrackSender::new(track_id.to_string(), TrackKind::Video)
        }
    }
}

impl Default for RTCPeerConnectionFactory {
    fn default() -> Self { Self::new() }
}
