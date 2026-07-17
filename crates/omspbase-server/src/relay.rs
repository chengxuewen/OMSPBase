use omspbase_core::error::CoreError;

#[cfg(feature = "webrtc")]
mod imp {
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use omspbase_core::error::CoreError;
    use omspbase_webrtc::{PeerConnection, PeerConnectionFactory, PcConfig};

    struct RelaySession {
        host_peer: Option<PeerConnection>,
        remote_peers: Vec<PeerConnection>,
    }

    pub struct Relay {
        factory: PeerConnectionFactory,
        sessions: Arc<Mutex<HashMap<String, RelaySession>>>,
    }
    impl Relay {
        pub fn new() -> Self {
            let factory = PeerConnectionFactory::new();
            tracing::info!("WebRTC relay initialized (omspbase-webrtc / libwebrtc)");
            Relay {
                factory,
                sessions: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        pub fn register_host(
            &self,
            room_id: &str,
        ) -> Result<PeerConnection, CoreError> {
            let config = PcConfig::default();
            let pc = self
                .factory
                .create_peer_connection(config)
                .map_err(|e| CoreError::PeerConnectionFailure(e.to_string()))?;
            let mut sessions = self.sessions.lock().await;
            let session = sessions
                .entry(room_id.to_string())
                .or_insert_with(|| RelaySession {
                    host_peer: None,
                    remote_peers: Vec::new(),
                });
            session.host_peer = Some(pc.clone());
            tracing::info!("Host registered in room {}", room_id);
            Ok(pc)
        }

        pub fn register_remote(
            &self,
            room_id: &str,
        ) -> Result<PeerConnection, CoreError> {
            let config = PcConfig::default();
            let pc = self
                .factory
                .create_peer_connection(config)
                .map_err(|e| CoreError::PeerConnectionFailure(e.to_string()))?;
            let mut sessions = self.sessions.lock().await;
            let session = sessions
                .entry(room_id.to_string())
                .or_insert_with(|| RelaySession {
                    host_peer: None,
                    remote_peers: Vec::new(),
                });
            session.remote_peers.push(pc.clone());
            tracing::info!(
                "Remote registered in room {} (total: {})",
                room_id,
                session.remote_peers.len()
            );
            Ok(pc)
        }

        /// Bridge tracks: bind Host video track to Remote PC.
        pub fn bridge_tracks(
            &self,
            _host_pc: &PeerConnection,
            _remote_pc: &PeerConnection,
        ) -> Result<(), CoreError> {
            tracing::info!("WebRTC track bridging (ponytail: stub — needs live track wiring)");
            Ok(())
        }
    }
}

#[cfg(not(feature = "webrtc"))]
mod imp {
    use super::CoreError;

    pub struct Relay;

    impl Relay {
        pub fn new() -> Self {
            tracing::info!("WebRTC relay initialized (stub — signaling-only mode)");
            Relay
        }

        /// ponytail: stub bridge_tracks; requires webrtc feature for real PeerConnection access.
        pub async fn bridge_tracks(
            &self,
            _host_pc: &(),
            _remote_pc: &(),
        ) -> Result<(), CoreError> {
            tracing::debug!("bridge_tracks stub — enable webrtc feature for real relay");
            Ok(())
        }
    }
}

pub use imp::Relay;
